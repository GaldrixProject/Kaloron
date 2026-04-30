// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use bytes::Bytes;
use std::sync::MutexGuard;
use std::{array, mem, ptr::NonNull, sync::Mutex};

const CHUNK_ITEMS: usize = 24;

/// One chainable payload item stored inline inside the ring.
///
/// The item itself is the linked-list node. A live payload slot stores the bytes
/// to emit together with a pointer to the next ring item in the detached or
/// sealed chain. This means a chain can be built and traversed directly through
/// [`RingItem`] pointers without carrying separate chunk/slot metadata inside the
/// item payload.
enum RingItem {
    /// Slot is outside the currently live payload set.
    None,
    /// Slot contains one live payload item.
    Bytes {
        /// Payload returned by [`RingGather::poll_frame`].
        bytes: Bytes,
        /// Next item in either the builder-local detached frame chain or the
        /// global send chain.
        next: Option<NonNull<RingItem>>,
    },
}

/// One chunk in the gather ring.
///
/// The chunk layout mirrors the spread ring: fixed-size inline item storage plus
/// doubly linked chunk pointers. This keeps slot addresses stable and lets
/// growth/shrink operate by chunk splicing rather than item relocation.
pub(super) struct RingChunk {
    items: [RingItem; CHUNK_ITEMS],
    prev: NonNull<RingChunk>,
    next: NonNull<RingChunk>,
}

/// Logical position inside the ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cursor {
    chunk: NonNull<RingChunk>,
    index: usize,
}

/// Slot and chunk management for the gather ring.
///
/// This helper owns the contiguous active/free range bookkeeping and all
/// chunk-splicing logic. It is also the sole authority for translating between
/// cursors and inline [`RingItem`] pointers, allocating new payload slots,
/// clearing consumed ones, compacting the active range, and growing or shrinking
/// the underlying chunk ring.
struct RingSlots {
    /// Head of the active range.
    used: NonNull<RingChunk>,
    /// Head of the free range.
    free: NonNull<RingChunk>,
    /// Intra-chunk slot index for `used`.
    used_index: usize,
    /// Intra-chunk slot index for `free`.
    free_index: usize,
    /// Logical distance from `used` to `free`.
    ///
    /// This counts every non-`None` slot currently reserved in the ring, whether
    /// the node is still detached in a builder-local frame chain or already
    /// linked into the global send chain.
    range_len: usize,
    /// Number of chunks currently linked into the ring.
    chunk_count: usize,
}

impl RingSlots {
    /// Creates the slot manager with two empty chunks.
    ///
    /// The ring intentionally keeps at least two chunks alive so short idle
    /// periods do not immediately cause allocation churn. Both cursors start at
    /// the first slot of the first chunk, so the active range is initially empty.
    fn new() -> Self {
        let first = Self::allocate_chunk();
        let second = Self::allocate_chunk();

        // SAFETY: both pointers come from fresh allocations owned by this ring.
        unsafe {
            (*first.as_ptr()).prev = second;
            (*first.as_ptr()).next = second;
            (*second.as_ptr()).prev = first;
            (*second.as_ptr()).next = first;
        }

        Self {
            used: first,
            free: first,
            used_index: 0,
            free_index: 0,
            range_len: 0,
            chunk_count: 2,
        }
    }

    /// Returns the cursor at the front of the active range.
    fn used_cursor(&self) -> Cursor {
        Cursor {
            chunk: self.used,
            index: self.used_index,
        }
    }

    /// Returns the cursor at the front of the free range.
    fn free_cursor(&self) -> Cursor {
        Cursor {
            chunk: self.free,
            index: self.free_index,
        }
    }

    /// Updates the front of the active range.
    fn set_used_cursor(&mut self, cursor: Cursor) {
        self.used = cursor.chunk;
        self.used_index = cursor.index;
    }

    /// Updates the front of the free range.
    fn set_free_cursor(&mut self, cursor: Cursor) {
        self.free = cursor.chunk;
        self.free_index = cursor.index;
    }

    /// Returns the total number of physical slots currently owned by the ring.
    fn total_capacity(&self) -> usize {
        self.chunk_count * CHUNK_ITEMS
    }

    /// Returns the number of slots that can still be allocated without moving any
    /// live node.
    fn usable_capacity(&self) -> usize {
        self.total_capacity() - self.used_index
    }

    /// Advances `cursor` by `steps` logical slots, wrapping across chunks as
    /// needed.
    fn advance_cursor(mut cursor: Cursor, steps: usize) -> Cursor {
        let mut offset = cursor.index + steps;
        while offset >= CHUNK_ITEMS {
            offset -= CHUNK_ITEMS;
            // SAFETY: all chunks participate in the same circular linked list.
            cursor.chunk = unsafe { cursor.chunk.as_ref() }.next;
        }
        cursor.index = offset;
        cursor
    }

    /// Returns the item stored at `cursor`.
    fn item_ref<'a>(cursor: Cursor) -> &'a RingItem {
        // SAFETY: `cursor` always points into a live chunk owned by the ring.
        unsafe { &cursor.chunk.as_ref().items[cursor.index] }
    }

    /// Returns a mutable reference to the item stored at `cursor`.
    fn item_mut<'a>(cursor: Cursor) -> &'a mut RingItem {
        // SAFETY: `cursor` always points into a live chunk owned by the ring, and
        // callers must uphold unique mutable access for the addressed slot.
        unsafe { &mut (*cursor.chunk.as_ptr()).items[cursor.index] }
    }

    /// Returns the pointer to the item stored at `cursor`.
    fn item_ptr(cursor: Cursor) -> NonNull<RingItem> {
        NonNull::from(Self::item_ref(cursor))
    }

    /// Returns an immutable view of an item pointer.
    fn item_ref_ptr<'a>(item: NonNull<RingItem>) -> &'a RingItem {
        // SAFETY: callers only pass pointers that were originally returned by this
        // ring and the corresponding slot is still expected to be live.
        unsafe { item.as_ref() }
    }

    /// Returns a mutable view of an item pointer.
    fn item_mut_ptr<'a>(mut item: NonNull<RingItem>) -> &'a mut RingItem {
        // SAFETY: callers only pass pointers that were originally returned by this
        // ring and the corresponding slot is still expected to be live, and
        // callers must uphold unique mutable access for the addressed slot.
        unsafe { item.as_mut() }
    }

    /// Allocates one node at the front of the free range and returns its stable
    /// inline pointer.
    fn allocate_node(&mut self, bytes: Bytes) -> NonNull<RingItem> {
        self.ensure_ring_capacity();

        let cursor = self.free_cursor();
        debug_assert!(matches!(Self::item_ref(cursor), RingItem::None));

        *Self::item_mut(cursor) = RingItem::Bytes { bytes, next: None };

        let item_ptr = Self::item_ptr(cursor);

        self.range_len += 1;
        self.set_free_cursor(Self::advance_cursor(cursor, 1));
        item_ptr
    }

    /// Clears one ring item back to [`RingItem::None`].
    ///
    /// After clearing, if the slot was at the front of the active range, the
    /// active range is compacted rightward across all contiguous empty slots.
    fn clear_item(&mut self, item: NonNull<RingItem>) -> RingItem {
        let cleared_was_used_front = item == Self::item_ptr(self.used_cursor());
        let item = mem::replace(Self::item_mut_ptr(item), RingItem::None);

        self.range_len -= 1;

        if cleared_was_used_front {
            self.compact_used_range();
        }

        self.adjust_ring_capacity();
        item
    }

    /// Advances the front of the active range across all contiguous `None` items.
    fn compact_used_range(&mut self) {
        if self.range_len == 0 {
            self.set_used_cursor(self.free_cursor());
            return;
        }

        let mut used = self.used_cursor();
        while matches!(Self::item_ref(used), RingItem::None) {
            used = Self::advance_cursor(used, 1);
        }
        self.set_used_cursor(used);
    }

    /// Grows the ring by one chunk when no usable slot remains.
    fn ensure_ring_capacity(&mut self) {
        debug_assert!(self.range_len <= self.usable_capacity());

        if self.range_len == self.usable_capacity() {
            self.insert_preceding_chunk();
        }
    }

    /// Shrinks the ring toward roughly 75% of its current chunk count once more
    /// than half of the usable capacity is empty.
    ///
    /// Shrink is conservative. It removes only whole chunks that are completely
    /// empty and sit immediately before `used`, never relocates a live node, and
    /// never drops below two chunks.
    fn adjust_ring_capacity(&mut self) {
        if self.chunk_count <= 2 {
            return;
        }

        let usable_capacity = self.usable_capacity();
        if self.range_len.saturating_mul(2) >= usable_capacity {
            return;
        }

        let minimum_chunks = (self.range_len + self.used_index)
            .div_ceil(CHUNK_ITEMS)
            .max(2);
        let target_chunks = (self.chunk_count * 3 / 4).max(2).max(minimum_chunks);

        while self.chunk_count > target_chunks && self.can_remove_preceding_chunk() {
            self.remove_preceding_chunk();
        }
    }

    /// Allocates one empty chunk with placeholder links.
    fn allocate_chunk() -> NonNull<RingChunk> {
        let chunk = Box::new(RingChunk {
            items: array::from_fn(|_| RingItem::None),
            prev: NonNull::dangling(),
            next: NonNull::dangling(),
        });

        NonNull::from(Box::leak(chunk))
    }

    /// Inserts one empty chunk immediately before `used`.
    ///
    /// The inserted chunk becomes the new start of the free range. The `used`
    /// cursor itself does not move.
    fn insert_preceding_chunk(&mut self) {
        let inserted = Self::allocate_chunk();
        let used = self.used;
        // SAFETY: `used` is a live node in the ring.
        let previous = unsafe { used.as_ref() }.prev;

        // SAFETY: `inserted`, `used`, and `previous` are live nodes that are being
        // linked into the same circular doubly linked list.
        unsafe {
            (*inserted.as_ptr()).prev = previous;
            (*inserted.as_ptr()).next = used;
            (*previous.as_ptr()).next = inserted;
            (*used.as_ptr()).prev = inserted;
        }

        self.free = inserted;
        self.free_index = 0;
        self.chunk_count += 1;
    }

    /// Returns whether the chunk immediately before `used` may be removed.
    fn can_remove_preceding_chunk(&self) -> bool {
        if self.chunk_count <= 2 {
            return false;
        }

        let candidate = unsafe { self.used.as_ref() }.prev;
        Self::chunk_is_empty(candidate)
    }

    /// Removes the whole empty chunk immediately before `used`.
    fn remove_preceding_chunk(&mut self) {
        let used = self.used;
        // SAFETY: `used` is a live node in the ring.
        let removed = unsafe { used.as_ref() }.prev;
        // SAFETY: `removed` is a live node directly before `used`.
        let previous = unsafe { removed.as_ref() }.prev;

        debug_assert!(self.can_remove_preceding_chunk());
        debug_assert!(Self::chunk_is_empty(removed));

        // SAFETY: `removed` is linked directly between `previous` and `used`.
        unsafe {
            (*previous.as_ptr()).next = used;
            (*used.as_ptr()).prev = previous;
        }

        if self.free == removed {
            debug_assert_eq!(self.free_index, 0);
            self.free = used;
            self.free_index = 0;
        }

        self.chunk_count -= 1;

        // SAFETY: `removed` was allocated with `Box::leak`, has been detached from
        // the ring, and is dropped exactly once here.
        unsafe {
            drop(Box::from_raw(removed.as_ptr()));
        }
    }

    /// Returns `true` when `chunk` contains no live nodes.
    fn chunk_is_empty(chunk: NonNull<RingChunk>) -> bool {
        // SAFETY: `chunk` points to a live ring chunk.
        unsafe { chunk.as_ref() }
            .items
            .iter()
            .all(|item| matches!(item, RingItem::None))
    }

    /// Drops every chunk in the ring exactly once.
    fn deallocate_chunk_ring(start: NonNull<RingChunk>, chunk_count: usize) {
        let mut current = start;
        for _ in 0..chunk_count {
            // SAFETY: the ring is well-formed and each chunk is dropped once.
            let next = unsafe { current.as_ref() }.next;
            // SAFETY: each chunk originated from `Box::leak`.
            unsafe {
                drop(Box::from_raw(current.as_ptr()));
            }
            current = next;
        }
    }
}

impl Drop for RingSlots {
    /// Drops all remaining ring chunks together with any inline nodes still stored
    /// inside them.
    fn drop(&mut self) {
        Self::deallocate_chunk_ring(self.used, self.chunk_count);
    }
}

/// Global sealed-frame send queue built from inline [`RingItem`] links.
///
/// This helper owns only queue head/tail state. The underlying node storage and
/// reclamation remain in [`RingSlots`]. Joining a frame appends its detached
/// start/end pair to the queue tail, while polling only removes the current head
/// node and advances the queue state; slot reclamation is handled by
/// [`RingGather::poll_frame`].
struct SendChain {
    /// First node in the global send chain, if any.
    head: Option<NonNull<RingItem>>,
    /// Last node in the global send chain, if any.
    tail: Option<NonNull<RingItem>>,
}

impl SendChain {
    /// Creates an empty send queue.
    fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    /// Appends one fully built detached frame to the queue tail.
    ///
    /// `start` and `end` describe one detached frame chain whose final node must
    /// still terminate with `next == None`.
    fn join_frame(&mut self, start: NonNull<RingItem>, end: NonNull<RingItem>) {
        if let Some(tail) = self.tail {
            if let RingItem::Bytes { next, .. } = RingSlots::item_mut_ptr(tail) {
                *next = Some(start);
            }
        } else {
            self.head = Some(start);
        }
        self.tail = Some(end);
    }

    /// Pops one payload node from the queue front and returns its pointer.
    ///
    /// The next pointer is read before the queue state is advanced so traversal
    /// remains valid even though the caller will later clear the corresponding
    /// slot.
    fn poll(&mut self) -> Option<NonNull<RingItem>> {
        let head = self.head?;
        let next = match RingSlots::item_ref_ptr(head) {
            RingItem::Bytes { next, .. } => *next,
            RingItem::None => unreachable!("queued payloads are always live items"),
        };

        self.head = next;
        if next.is_none() {
            self.tail = None;
        }
        Some(head)
    }
}

/// Gather-side dynamic ring for outbound payload nodes.
///
/// Conceptually, the ring tracks one detached/sealed send queue whose payload
/// slots live inside [`RingSlots`]. Slot allocation, reclamation, and chunk
/// growth/shrink are delegated to that helper, while [`SendChain`] owns the
/// global sealed-frame queue. [`RingGather`] itself is focused on building
/// detached frame chains and handing them to the send queue.
pub(super) struct RingGather {
    /// Inline slot and chunk manager for payload nodes.
    slots: Mutex<RingSlots>,
    /// Global sealed-frame send queue.
    ///
    /// Lock ordering rule: when an operation needs both `slots` and `send`, it
    /// must always lock `slots` first and `send` second.
    send: Mutex<SendChain>,
}

impl RingGather {
    /// Creates the gather ring with two empty chunks.
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(RingSlots::new()),
            send: Mutex::new(SendChain::new()),
        }
    }

    fn lock_slots(&'_ self) -> MutexGuard<'_, RingSlots> {
        self.slots.lock().expect("gather ring slots mutex poisoned")
    }

    fn lock_send(&'_ self) -> MutexGuard<'_, SendChain> {
        self.send.lock().expect("gather ring send mutex poisoned")
    }

    /// Starts building a new detached frame.
    ///
    /// The returned builder is tied to this ring and appends payload items
    /// directly into the ring's slot storage. The frame is not made visible to
    /// the send side until the builder is sealed.
    pub fn new_frame(&mut self) -> FrameBuilder {
        FrameBuilder::new(self)
    }

    /// Allocates the first item of a detached frame chain.
    ///
    /// This reserves one slot at the front of the free range, writes `bytes` into
    /// it, and returns a stable pointer to the inline [`RingItem`]. The returned
    /// pointer becomes both the start and end of the builder-local frame chain.
    fn start_frame(&mut self, bytes: Bytes) -> NonNull<RingItem> {
        self.lock_slots().allocate_node(bytes)
    }

    /// Appends one node to a detached frame chain whose current tail is `tail`.
    ///
    /// The implementation allocates one new ring slot, links `tail.next` to the
    /// new node, and returns the new tail pointer. This does not attach the chain
    /// to the global send queue yet.
    fn chain_frame(&mut self, tail: NonNull<RingItem>, bytes: Bytes) -> NonNull<RingItem> {
        let new_tail = self.lock_slots().allocate_node(bytes);
        match RingSlots::item_mut_ptr(tail) {
            RingItem::Bytes { next, .. } => *next = Some(new_tail),
            RingItem::None => unreachable!("builder only chains live frame items"),
        }
        new_tail
    }

    /// Attaches a detached frame chain to the end of the global send chain.
    ///
    /// `start` and `end` describe one fully built detached frame. Attaching the
    /// frame only updates send-chain links; it never relocates any node.
    fn seal_frame(&mut self, start: NonNull<RingItem>, end: NonNull<RingItem>) {
        self.lock_send().join_frame(start, end);
    }

    /// Drops a detached frame chain instead of queueing it for send.
    ///
    /// Every node from `start` through `end` is cleared back to [`RingItem::None`].
    /// After clearing each slot, if that slot is currently the front of the active
    /// range, `used` is advanced rightward across all contiguous `None` items.
    fn drop_frame(&mut self, start: NonNull<RingItem>, end: NonNull<RingItem>) {
        let mut slots = self.lock_slots();
        let mut current = start;
        loop {
            let next = match RingSlots::item_ref_ptr(current) {
                RingItem::Bytes { next, .. } => *next,
                RingItem::None => unreachable!("builder only drops live detached frame items"),
            };
            let _ = slots.clear_item(current);
            if current == end {
                break;
            }
            current = next.expect("detached frame chain unexpectedly terminated early");
        }
    }

    /// Pops one payload from the front of the global send chain.
    ///
    /// On success this returns the next queued [`Bytes`] value, clears the
    /// corresponding ring slot back to [`RingItem::None`], compacts the active
    /// range if the front became empty, and applies the normal shrink heuristic.
    ///
    /// Returns `None` when no queued payload is currently available.
    pub fn poll_frame(&mut self) -> Option<Bytes> {
        let head = self.lock_send().poll()?;
        match self.lock_slots().clear_item(head) {
            RingItem::Bytes { bytes, .. } => Some(bytes),
            RingItem::None => unreachable!("queued payloads are always live items"),
        }
    }
}

/// Builder-local frame assembly state returned by [`RingGather::new_frame`].
///
/// A `FrameBuilder` stores only detached chain endpoints and a raw pointer back
/// to the ring that created it. The builder itself does not expose any public
/// constructor; callers obtain it from [`RingGather::new_frame`], use
/// [`FrameBuilder::append_frame`] to add payload items, and finish with
/// [`FrameBuilder::seal_frame`] or by letting the builder drop while still
/// building.
///
/// The ring owns the slot storage; the builder only records the start and end of
/// its in-progress chain so it can later seal or drop that chain in one go.
pub(super) struct FrameBuilder {
    /// Ring instance this builder operates on.
    ring: NonNull<RingGather>,
    /// First node in the detached frame chain currently being built.
    start: Option<NonNull<RingItem>>,
    /// Last node in the detached frame chain currently being built.
    end: Option<NonNull<RingItem>>,
}

impl FrameBuilder {
    /// Internal constructor used by [`RingGather::new_frame`].
    ///
    /// The builder stores the ring address as a raw pointer so the public ring
    /// API can hand out lightweight frame builders without exposing any other
    /// creation path. The caller must ensure the ring outlives the builder and is
    /// not moved while the builder is still in use.
    fn new(ring: &mut RingGather) -> Self {
        Self {
            ring: NonNull::from(ring),
            start: None,
            end: None,
        }
    }

    /// Returns a mutable reference to the bound ring.
    fn ring(&mut self) -> &mut RingGather {
        // SAFETY: `FrameBuilder` stores a pointer to a live `RingGather`, and the
        // caller must keep that ring alive for the whole builder lifetime.
        unsafe { self.ring.as_mut() }
    }

    /// Advances the builder by one node, starting a detached frame or extending
    /// the current one as needed.
    pub fn append_frame(&mut self, bytes: Bytes) {
        if self.start.is_none() {
            let node = self.ring().start_frame(bytes);
            self.start = Some(node);
            self.end = Some(node);
        } else {
            let end = self.end.expect("builder end state is inconsistent");
            let new_end = self.ring().chain_frame(end, bytes);
            self.end = Some(new_end);
        }
    }

    /// Seals the detached frame and appends it to the ring's global send chain.
    ///
    /// If the builder is currently empty, this is a no-op. Otherwise the current
    /// detached frame is sealed into the ring's global send chain and the builder
    /// is consumed so it cannot be reused afterward.
    pub fn seal_frame(self) {
        let mut this = self;
        let Some(start) = this.start else {
            return;
        };
        let end = this.end.expect("builder end state is inconsistent");

        this.ring().seal_frame(start, end);
        mem::forget(this);
    }
}

impl Drop for FrameBuilder {
    /// Drops any detached frame that is still being built.
    ///
    /// This makes frame cleanup automatic for the common case where a builder is
    /// simply abandoned before sealing. If the builder is already empty, dropping
    /// it is a no-op. Sealed frames are left alone because they are already part
    /// of the global send chain.
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let end = self.end.expect("builder end state is inconsistent");
            self.ring().drop_frame(start, end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    fn assist_byte_item(ch: u8) -> Bytes {
        Bytes::from(vec![ch])
    }

    #[test]
    fn test_size_1k_per_node_constraint() {
        let size = size_of::<RingChunk>();
        // CRITICAL: this is to ensure fill rate on future 1K pool optimization.
        // this is a requirement, adjust CHUNK_ITEMS to make this happen.
        assert!(size > 944 && size < 984)
    }

    #[test]
    fn test_builder_sealed_frames_poll_in_payload_order() {
        let mut ring = RingGather::new();
        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'a'));
            builder.append_frame(assist_byte_item(b'b'));
            builder.seal_frame();
        }

        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'c'));
            builder.seal_frame();
        }

        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'a')));
        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'b')));
        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'c')));
        assert_eq!(ring.poll_frame(), None);
    }

    #[test]
    fn test_dropping_detached_frame_reclaims_nodes_without_queueing() {
        let mut ring = RingGather::new();

        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'x'));
            builder.append_frame(assist_byte_item(b'y'));
        }

        assert_eq!(ring.slots.lock().unwrap().range_len, 0);
        assert_eq!(ring.send.lock().unwrap().head, None);
        assert_eq!(ring.send.lock().unwrap().tail, None);
        assert_eq!(ring.poll_frame(), None);
    }

    #[test]
    fn test_ring_grows_and_shrinks_back_to_two_chunks() {
        let mut ring = RingGather::new();
        let mut builders = Vec::new();

        for index in 0..(CHUNK_ITEMS * 2 + 1) {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(index as u8));
            builder.seal_frame();
            builders.push(index as u8);
        }

        assert_eq!(ring.slots.lock().unwrap().chunk_count, 3);

        for byte in builders {
            assert_eq!(ring.poll_frame(), Some(assist_byte_item(byte)));
        }

        assert_eq!(ring.poll_frame(), None);
        {
            let slots = ring.slots.lock().unwrap();
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.chunk_count, 2);
            assert_eq!(slots.used_cursor(), slots.free_cursor());
        }
    }

    #[test]
    fn test_builder_drop_cleans_up_unsealed_frame_and_new_builder_can_continue() {
        let mut ring = RingGather::new();

        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'a'));
            builder.seal_frame();
        }

        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'b'));
        }

        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'a')));
        assert_eq!(ring.poll_frame(), None);

        {
            let mut builder = ring.new_frame();
            builder.append_frame(assist_byte_item(b'c'));
            builder.seal_frame();
        }

        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'c')));
        assert_eq!(ring.poll_frame(), None);
    }

    #[test]
    fn test_builder_empty_seal_and_drop_are_noops() {
        let mut ring = RingGather::new();

        ring.new_frame().seal_frame();
        assert_eq!(ring.poll_frame(), None);
        {
            let slots = ring.slots.lock().unwrap();
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.chunk_count, 2);
        }

        drop(ring.new_frame());
        assert_eq!(ring.poll_frame(), None);
        {
            let slots = ring.slots.lock().unwrap();
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.chunk_count, 2);
        }
    }

    #[test]
    fn test_builder_append_frame_can_be_called_twice_to_append() {
        let mut ring = RingGather::new();
        let mut builder = ring.new_frame();

        builder.append_frame(assist_byte_item(b'a'));
        builder.append_frame(assist_byte_item(b'b'));
        builder.seal_frame();

        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'a')));
        assert_eq!(ring.poll_frame(), Some(assist_byte_item(b'b')));
        assert_eq!(ring.poll_frame(), None);
    }
}
