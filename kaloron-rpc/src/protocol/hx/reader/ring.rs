// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use bytes::Bytes;
use std::sync::{Mutex, MutexGuard};
use std::{array, mem, ptr::NonNull};

const CHUNK_ITEMS: usize = 24;

pub(super) struct Frame {
    // chunk that stores this frame header
    chunk: NonNull<RingChunk>,
    // number of `Bytes` items that belong to this frame
    byte_items: u32,
    // slot index of this frame header inside `chunk`
    slot: u16,
    // whether the frame was freed before reaching the front of the active range
    is_free: bool,
}

pub(super) enum FrameRead {
    /// requested buffer fully filled
    Full,
    /// only a part of the data is read due to end of chunk
    /// this is returned even is buffer is fully filled byt the right side hits chunk boundary.
    Trim(u32),
    /// end of the frame
    EoF,
}

impl Frame {
    pub(super) fn read(&self, item: u32, byte: u32, buf: &mut [u8]) -> FrameRead {
        // any read beyond past the end of items returns EoF
        if item >= self.byte_items {
            return FrameRead::EoF;
        }
        // now we locate the byte slice being requested.
        let mut chunk = self.chunk;
        // fix the offset based on start of chunk to allow unified lookup
        let mut offset = item + self.slot as u32 + 1;
        while offset >= CHUNK_ITEMS as u32 {
            offset -= CHUNK_ITEMS as u32;
            // SAFETY: all chunks participate in the same circular linked list.
            chunk = unsafe { chunk.as_ref() }.next;
        }
        // we have located the offset.
        // this has to be a Bytes item as guaranteed by the ring behavior.
        let slot = &mut unsafe { chunk.as_mut() }.items[offset as usize];
        // this is lifted out for a reason. see fallthrough below.
        let remain: u32;
        match slot {
            RingItem::None | RingItem::Frame(_) => {
                panic!("ring corrupted or accessing consumed data")
            }
            RingItem::Bytes(b) => {
                let len = b.len() as u32;
                if byte >= len {
                    // we treat attempt to read chunk boundary also as out of bound.
                    // this is because chunk advancing should be handled upon receiving Trim.
                    // not properly doing so is a bug
                    panic!("attempted to read beyond the frame payload boundary");
                }
                // SAFETY: bound check already done
                let (_, w) = unsafe { b.split_at_unchecked(byte as usize) };
                remain = len - byte;
                if remain > buf.len() as u32 {
                    buf.copy_from_slice(&w[..buf.len()]);
                    return FrameRead::Full;
                } else {
                    buf[..(remain as usize)].copy_from_slice(w);
                    // fallthrough to outside of match to access the slot reference
                    // we still need remain for return value so remain is lifted out
                }
            }
        }
        // we are dropping the content of the bytes here.
        // this is so that when releasing the frame, if there is a mutex needed,
        // the mutex will not need to be held too long for releasing the buffer.
        // this could potentially lower contention.
        // Bytes::clear cannot be used here because if is simply truncate to 0 which does not drop.
        // Bytes::new() cannot be used here because if still calls drop via ptr.
        // this is still redirection using two ptr read even though the static drop is no-op.
        // though this still may cause cache pollution on reclaim pass, it is not as bad as Bytes.
        *slot = RingItem::None;
        FrameRead::Trim(remain)
    }
}

enum RingItem {
    None,
    Frame(Frame),
    Bytes(Bytes),
}

struct RingChunk {
    items: [RingItem; CHUNK_ITEMS],
    prev: NonNull<RingChunk>,
    next: NonNull<RingChunk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cursor {
    chunk: NonNull<RingChunk>,
    index: usize,
}

/// Slot and chunk management for the spread ring.
///
/// This helper owns the contiguous active/free range bookkeeping and all
/// chunk-splicing logic. It is also the sole authority for translating between
/// cursors and inline frame/payload slots, clearing consumed payload storage,
/// reclaiming front-aligned frames, and growing or shrinking the underlying
/// chunk ring.
struct RingSlots {
    // head of the active range
    used: NonNull<RingChunk>,
    // head of the free range
    free: NonNull<RingChunk>,
    used_index: usize,
    free_index: usize,
    // logical distance from `used` to `free`; this includes frame headers that
    // were marked free but have not yet reached the front of the range
    range_len: usize,
    chunk_count: usize,
}

impl RingSlots {
    /// Creates the slot manager with two empty chunks.
    ///
    /// The ring intentionally keeps at least two chunks alive so short-lived idle
    /// periods do not immediately cause chunk allocation churn. Both cursors start
    /// at the first slot of the first chunk, so the active range is initially
    /// empty.
    fn new() -> Self {
        let first = Self::allocate_chunk();
        let second = Self::allocate_chunk();

        // SAFETY: both pointers come from fresh chunk allocations owned by this
        // ring during construction.
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

    /// Returns the number of slots that may be appended to without relocating any
    /// existing item.
    fn usable_capacity(&self) -> usize {
        self.total_capacity() - self.used_index
    }

    /// Advances `cursor` by `steps` logical slots, wrapping across chunk
    /// boundaries as needed.
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
    fn item_ref(&self, cursor: Cursor) -> &RingItem {
        // SAFETY: `cursor` always points into a live chunk owned by the ring.
        unsafe { &cursor.chunk.as_ref().items[cursor.index] }
    }

    /// Returns a mutable reference to the item stored at `cursor`.
    fn item_mut(&mut self, cursor: Cursor) -> &mut RingItem {
        // SAFETY: `cursor` always points into a live chunk owned by the ring, and
        // `&mut self` guarantees unique mutable access.
        unsafe { &mut (*cursor.chunk.as_ptr()).items[cursor.index] }
    }

    /// Replaces the item at `cursor` with [`RingItem::None`] and returns the old
    /// value.
    fn take_item(&mut self, cursor: Cursor) -> RingItem {
        mem::replace(self.item_mut(cursor), RingItem::None)
    }

    /// Grows the ring by one chunk when no usable slot remains.
    ///
    /// Usable capacity excludes the prefix of the `used` chunk before
    /// `used_index`. Re-entering that prefix would require item relocation, so the
    /// ring grows instead by inserting one empty chunk immediately before `used`.
    /// The `used` cursor itself is unchanged.
    fn ensure_ring_capacity(&mut self) {
        debug_assert!(self.range_len <= self.usable_capacity());

        if self.range_len == self.usable_capacity() {
            self.insert_preceding_chunk();
        }
    }

    /// Appends one item at the front of the free range and returns its cursor.
    ///
    /// Capacity growth, slot reservation, location-dependent frame metadata
    /// fixup, and slot assignment are all handled here so callers only need to
    /// supply the logical item they want to append.
    fn append(&mut self, mut item: RingItem) -> Cursor {
        self.ensure_ring_capacity();

        let cursor = self.free_cursor();
        debug_assert!(matches!(self.item_ref(cursor), RingItem::None));

        if let RingItem::Frame(frame) = &mut item {
            frame.chunk = cursor.chunk;
            frame.slot = cursor.index as u16;
        }

        *self.item_mut(cursor) = item;
        self.range_len += 1;
        self.set_free_cursor(Self::advance_cursor(cursor, 1));
        cursor
    }

    /// Shrinks the ring toward roughly 75% of its current chunk count once more
    /// than half of the usable capacity is empty.
    ///
    /// Shrink is conservative. It removes only whole chunks that are completely
    /// empty and sit immediately before `used`, never relocates a live item, and
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

    /// Validates that `cursor` still stores `frame` and returns the frame's payload
    /// length.
    fn frame_byte_count(&self, cursor: Cursor, frame: NonNull<Frame>) -> usize {
        match self.item_ref(cursor) {
            RingItem::Frame(stored) if NonNull::from(stored) == frame => {
                if stored.is_free {
                    panic!("frame has already been freed");
                }
                stored.byte_items as usize
            }
            RingItem::Frame(_) => {
                panic!("frame pointer does not match the frame stored in its slot")
            }
            RingItem::None => panic!("frame slot has already been reclaimed"),
            RingItem::Bytes(_) => panic!("frame pointer refers to a non-frame slot"),
        }
    }

    /// Clears the payload items owned by the frame stored at `frame_cursor`.
    fn clear_frame_bytes(&mut self, frame_cursor: Cursor, byte_count: usize) {
        let mut cursor = Self::advance_cursor(frame_cursor, 1);
        for _ in 0..byte_count {
            *self.item_mut(cursor) = RingItem::None;
            cursor = Self::advance_cursor(cursor, 1);
        }
    }

    /// Reclaims the frame at the front of the active range.
    ///
    /// `byte_count` must match the payload length of the frame currently addressed
    /// by [`used_cursor`](Self::used_cursor).
    fn reclaim_front_frame(&mut self, byte_count: usize) {
        let used = self.used_cursor();
        match self.take_item(used) {
            RingItem::Frame(_) => {}
            RingItem::None => panic!("attempted to reclaim an empty slot as a frame"),
            RingItem::Bytes(_) => panic!("attempted to reclaim bytes as a frame header"),
        }

        self.range_len -= 1 + byte_count;
        let next_used = Self::advance_cursor(used, 1 + byte_count);
        self.set_used_cursor(next_used);

        if self.range_len == 0 {
            self.set_free_cursor(next_used);
        }
    }

    /// Continues reclaiming consecutive freed headers that have reached the front
    /// of the active range.
    fn reclaim_following_freed_frames(&mut self) {
        while self.range_len > 0 {
            let used = self.used_cursor();
            let byte_count = match self.item_ref(used) {
                RingItem::Frame(frame) if frame.is_free => frame.byte_items as usize,
                RingItem::Frame(_) => break,
                RingItem::None => panic!("active range starts with an empty slot"),
                RingItem::Bytes(_) => panic!("active range starts in the middle of frame payload"),
            };

            self.clear_frame_bytes(used, byte_count);
            self.reclaim_front_frame(byte_count);
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
    /// cursor does not move.
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

    /// Returns whether the chunk immediately before `used` is both removable and
    /// currently empty.
    fn can_remove_preceding_chunk(&self) -> bool {
        if self.chunk_count <= 2 {
            return false;
        }

        let candidate = unsafe { self.used.as_ref() }.prev;
        Self::chunk_is_empty(candidate)
    }

    /// Removes the whole chunk immediately before `used`.
    ///
    /// The removed chunk must already be empty. If it also happens to be the free
    /// chunk, the free cursor moves to the start of `used`.
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

    /// Returns `true` if every slot in `chunk` is [`RingItem::None`].
    fn chunk_is_empty(chunk: NonNull<RingChunk>) -> bool {
        // SAFETY: `chunk` points to a live ring chunk.
        unsafe { chunk.as_ref() }
            .items
            .iter()
            .all(|item| matches!(item, RingItem::None))
    }

    /// Drops every chunk in the circular chunk list exactly once.
    fn deallocate_chunk_ring(start: NonNull<RingChunk>, chunk_count: usize) {
        let mut current = start;
        for _ in 0..chunk_count {
            // SAFETY: the chunk list is well-formed and each chunk is dropped once.
            let next = unsafe { current.as_ref().next };
            // SAFETY: each chunk originated from `Box::leak`.
            unsafe {
                drop(Box::from_raw(current.as_ptr()));
            }
            current = next;
        }
    }
}

impl Drop for RingSlots {
    /// Drops the ring together with all remaining chunks, frame headers, and byte
    /// payloads still stored inside them.
    fn drop(&mut self) {
        Self::deallocate_chunk_ring(self.used, self.chunk_count);
    }
}

enum RingState {
    ReadyForFrame,
    BuildingFrame(NonNull<Frame>),
}

pub(super) struct RingSpread {
    /// Inline slot and chunk manager for frame headers and payload items.
    slots: Mutex<RingSlots>,
    state: RingState,
}

impl RingSpread {
    /// Creates a ring with two empty chunks.
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(RingSlots::new()),
            state: RingState::ReadyForFrame,
        }
    }

    fn lock_slots(&'_ self) -> MutexGuard<'_, RingSlots> {
        self.slots.lock().expect("spread ring slots mutex poisoned")
    }

    /// Starts a new frame at the current free cursor.
    ///
    /// This reserves exactly one ring slot for the frame header, records where the
    /// header lives, and transitions the ring into the frame-building state. While
    /// a frame is open, the only legal follow-up operations are
    /// [`push_bytes`](Self::push_bytes) and [`seal_frame`](Self::seal_frame).
    ///
    /// If no usable slot remains, the ring first grows by inserting one brand new
    /// empty chunk immediately before `used`. Growth never relocates any existing
    /// item, so previously returned frame pointers remain valid.
    ///
    /// # Panics
    ///
    /// Panics if another frame is already open.
    pub fn push_frame(&mut self) -> NonNull<Frame> {
        if !matches!(self.state, RingState::ReadyForFrame) {
            panic!("cannot push a new frame while another frame is open");
        }

        let frame_ptr = {
            let mut slots = self.lock_slots();
            let cursor = slots.append(RingItem::Frame(Frame {
                chunk: NonNull::dangling(),
                byte_items: 0,
                slot: 0,
                is_free: false,
            }));

            match slots.item_mut(cursor) {
                RingItem::Frame(frame) => NonNull::from(frame),
                RingItem::None | RingItem::Bytes(_) => {
                    unreachable!("frame slot was just initialized")
                }
            }
        };

        self.state = RingState::BuildingFrame(frame_ptr);
        frame_ptr
    }

    /// Appends one [`Bytes`] item to the currently open frame.
    ///
    /// Each call writes `bytes` into the next free slot, advances the free cursor,
    /// and increments the open frame's payload count. The count is later used by
    /// [`free_frame`](Self::free_frame) to clear the exact payload span that belongs
    /// to the frame.
    ///
    /// If the ring runs out of usable slots, it grows by splicing a fresh empty
    /// chunk immediately before `used`. Existing items are never relocated.
    ///
    /// # Panics
    ///
    /// Panics if there is no frame open.
    pub fn push_bytes(&mut self, bytes: Bytes) {
        let frame = match self.state {
            RingState::BuildingFrame(frame) => frame,
            RingState::ReadyForFrame => panic!("cannot push bytes without an open frame"),
        };

        {
            let mut slots = self.lock_slots();
            slots.append(RingItem::Bytes(bytes));

            // SAFETY: `frame` points at an inline frame that remains in place until it
            // is reclaimed. Chunk growth and shrink only splice whole empty chunks.
            unsafe {
                let frame = frame.as_ptr();
                (*frame).byte_items = (*frame)
                    .byte_items
                    .checked_add(1)
                    .expect("frame byte item count overflowed u32");
            }
        }
    }

    /// Seals the currently open frame and returns a stable pointer to its header.
    ///
    /// After sealing, the ring goes back to the state where only
    /// [`push_frame`](Self::push_frame) is allowed. The returned pointer remains
    /// valid until the frame is reclaimed by [`free_frame`](Self::free_frame) or
    /// the entire ring is dropped.
    ///
    /// # Panics
    ///
    /// Panics if no frame is open.
    pub fn seal_frame(&mut self) -> NonNull<Frame> {
        match mem::replace(&mut self.state, RingState::ReadyForFrame) {
            RingState::BuildingFrame(frame) => frame,
            RingState::ReadyForFrame => panic!("cannot seal a frame when none is open"),
        }
    }

    /// Frees a previously sealed frame.
    ///
    /// The frame's trailing [`Bytes`] items are always cleared immediately. If the
    /// frame header is also the first item in the active range, the header itself
    /// is reclaimed right away and `used` advances past the whole frame. Any
    /// consecutive freed headers that are now at the front are reclaimed in the
    /// same pass.
    ///
    /// If the frame is not yet at the front of the active range, its header stays
    /// in place and is only marked as free. That deferred header will be reclaimed
    /// later once all earlier frames have been released.
    ///
    /// After reclaiming, the ring may shrink by unlinking whole empty chunks that
    /// sit immediately before `used`. Shrink never relocates live items and never
    /// reduces the ring below two chunks.
    ///
    /// # Panics
    ///
    /// Panics if `frame` does not refer to a live frame owned by this ring, or if
    /// the caller attempts to free the same deferred frame twice.
    ///
    /// # Safety contract for callers
    ///
    /// `frame` must be a pointer previously returned by
    /// [`seal_frame`](Self::seal_frame) for this ring and must still point at a
    /// live frame header.
    pub fn free_frame(&self, frame: NonNull<Frame>) {
        let mut slots = self.lock_slots();
        let frame_cursor = {
            // SAFETY: the public contract requires `frame` to point at a live frame
            // that belongs to this ring.
            let frame_ref = unsafe { frame.as_ref() };
            Cursor {
                chunk: frame_ref.chunk,
                index: frame_ref.slot as usize,
            }
        };

        let byte_count = slots.frame_byte_count(frame_cursor, frame);
        slots.clear_frame_bytes(frame_cursor, byte_count);

        if frame_cursor == slots.used_cursor() {
            slots.reclaim_front_frame(byte_count);
            slots.reclaim_following_freed_frames();
        } else {
            // SAFETY: `frame` was validated against the slot contents above.
            unsafe {
                (*frame.as_ptr()).is_free = true;
            }
        }

        slots.adjust_ring_capacity();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assist_byte_item() -> Bytes {
        Bytes::from_static(b"x")
    }

    fn assist_seal_empty_frame(ring: &mut RingSpread) -> NonNull<Frame> {
        ring.push_frame();
        ring.seal_frame()
    }

    fn assist_seal_frame_with_payload(ring: &mut RingSpread, byte_items: usize) -> NonNull<Frame> {
        ring.push_frame();
        for _ in 0..byte_items {
            ring.push_bytes(assist_byte_item());
        }
        ring.seal_frame()
    }

    fn assist_frame_cursor(frame: NonNull<Frame>) -> Cursor {
        // SAFETY: tests only call this helper while the frame header is still live.
        let frame = unsafe { frame.as_ref() };
        Cursor {
            chunk: frame.chunk,
            index: frame.slot as usize,
        }
    }

    #[test]
    fn test_size_1k_per_node_constraint() {
        let size = size_of::<RingChunk>();
        assert!(size > 944 && size < 984)
    }

    #[test]
    fn test_empty_front_frame_round_trips_back_to_empty_ring() {
        let mut ring = RingSpread::new();
        let frame = assist_seal_empty_frame(&mut ring);

        ring.free_frame(frame);

        {
            let slots = ring.lock_slots();
            assert_eq!(slots.chunk_count, 2);
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.used_cursor(), slots.free_cursor());
            assert_eq!(slots.used_index, 1);
        }
    }

    #[test]
    fn test_freeing_later_frame_defers_header_reclaim_until_front_advances() {
        let mut ring = RingSpread::new();
        let first = assist_seal_frame_with_payload(&mut ring, 1);
        let second = assist_seal_frame_with_payload(&mut ring, 2);

        ring.free_frame(second);

        let second_cursor = assist_frame_cursor(second);
        {
            let slots = ring.lock_slots();
            match slots.item_ref(second_cursor) {
                RingItem::Frame(frame) => {
                    assert!(frame.is_free);
                    assert_eq!(frame.byte_items, 2);
                }
                RingItem::None | RingItem::Bytes(_) => {
                    panic!("expected deferred frame header to remain in place")
                }
            }
            assert!(matches!(
                slots.item_ref(RingSlots::advance_cursor(second_cursor, 1)),
                RingItem::None
            ));
            assert!(matches!(
                slots.item_ref(RingSlots::advance_cursor(second_cursor, 2)),
                RingItem::None
            ));
        }

        ring.free_frame(first);

        {
            let slots = ring.lock_slots();
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.used_cursor(), slots.free_cursor());
        }
    }

    #[test]
    fn test_ring_grows_and_then_shrinks_back_to_two_chunks() {
        let mut ring = RingSpread::new();
        let mut frames = Vec::new();

        for _ in 0..(CHUNK_ITEMS * 2 + 1) {
            frames.push(assist_seal_empty_frame(&mut ring));
        }

        assert_eq!(ring.lock_slots().chunk_count, 3);

        for frame in frames {
            ring.free_frame(frame);
        }

        {
            let slots = ring.lock_slots();
            assert_eq!(slots.chunk_count, 2);
            assert_eq!(slots.range_len, 0);
            assert_eq!(slots.used_cursor(), slots.free_cursor());
        }
    }

    #[test]
    #[should_panic(expected = "cannot push bytes without an open frame")]
    fn test_push_bytes_without_an_open_frame_panics() {
        let mut ring = RingSpread::new();
        ring.push_bytes(assist_byte_item());
    }

    #[test]
    #[should_panic(expected = "cannot seal a frame when none is open")]
    fn test_sealing_without_an_open_frame_panics() {
        let mut ring = RingSpread::new();
        let _ = ring.seal_frame();
    }

    #[test]
    #[should_panic(expected = "cannot push a new frame while another frame is open")]
    fn test_opening_a_second_frame_before_sealing_panics() {
        let mut ring = RingSpread::new();
        ring.push_frame();
        ring.push_frame();
    }

    #[test]
    #[should_panic(expected = "frame has already been freed")]
    fn test_freeing_a_deferred_frame_twice_panics() {
        let mut ring = RingSpread::new();
        let _first = assist_seal_empty_frame(&mut ring);
        let second = assist_seal_empty_frame(&mut ring);

        ring.free_frame(second);
        ring.free_frame(second);
    }
}
