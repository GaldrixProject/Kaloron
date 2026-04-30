Kaloron Visitor / Transmit Design
=================================

Status: implemented

Overview
--------
This document describes the visitor-based send/recv protocol used by kaloron
to traverse values according to their `Schema`. The design uses a small set
of paired traits — one for sending (reading values) and one for receiving
(constructing values) — that compose to handle all schema shapes.

The visitor protocol is independent of any specific wire format. Concrete
transport implementations (e.g., `kaloron-rpc/src/wire/`) use these traits
to bridge between typed Rust values and byte streams.

Architecture
------------

```
                  TypeShape::send                    TypeShape::recv
                      │                                    │
                      ▼                                    ▼
               ┌─────────────┐                    ┌──────────────┐
               │ SendAccept  │                    │  RecvAccept  │
               └──┬──────────┘                    └──┬───────────┘
                  │ dispatches to                     │ dispatches to
                  ▼                                   ▼
         ┌────────────────┐                   ┌────────────────┐
         │  *Send traits  │                   │  *Recv traits  │
         │ PrimitiveSend  │                   │ PrimitiveRecv  │
         │ OptionSend     │                   │ OptionRecv     │
         │ SequenceSend   │                   │ SequenceRecv   │
         │ TupleSend      │                   │ TupleRecv      │
         │ MapSend        │                   │ MapRecv        │
         │ NewTypeSend    │                   │ NewTypeRecv    │
         │ EnumSend       │                   │ EnumRecv       │
         └────────┬───────┘                   └────────┬───────┘
                  │ calls                              │ calls
                  ▼                                    ▼
         ┌────────────────┐                   ┌────────────────┐
         │  SendVisitor   │                   │  RecvVisitor   │
         │  visit<T>(&T)  │                   │  visit<T>() →T │
         └────────────────┘                   └────────────────┘
```

Core visitor traits
-------------------

### SendVisitor / RecvVisitor

The fundamental visitor interfaces for individual values:

```rust
pub trait SendVisitor {
    fn visit<T>(&mut self, value: &T) -> anyhow::Result<()>;
}

pub trait RecvVisitor {
    fn visit<T>(&mut self) -> anyhow::Result<T>;
}
```

`SendVisitor::visit` receives a shared reference to a value and forwards it
to the transport. `RecvVisitor::visit` produces an owned value from the
transport.

Schema-specific send traits
----------------------------

Each schema shape has a dedicated send trait that describes how to extract
data from a value of that shape.

### PrimitiveSend

```rust
pub trait PrimitiveSend {
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}
```

Forwards a single primitive value to the visitor.

### OptionSend

```rust
pub trait OptionSend {
    fn is_some(&self) -> bool;
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}
```

`is_some()` indicates presence; `visit()` forwards the inner value when present.

### NewTypeSend

```rust
pub trait NewTypeSend {
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}
```

Forwards the single wrapped value of a newtype struct.

### SequenceSend

```rust
pub trait SequenceSend {
    fn len(&self) -> Option<usize>;
    fn has_next(&self) -> bool;
    fn visit_next(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}
```

Iterates over a variable-length sequence. `len()` provides an optional
size hint. `has_next()` / `visit_next()` form an iterator-style protocol.

### TupleSend

```rust
pub trait TupleSend {
    fn visit(&self, index: isize, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}
```

Positional access for tuples, tuple structs, and named struct fields.
The `index` is a zero-based field/element position. For named structs,
fields are visited in id order.

### MapSend

```rust
pub trait MapSend {
    fn len(&self) -> Option<usize>;
    fn has_next(&self) -> bool;
    fn visit_next(
        &self,
        visitor_k: &mut impl SendVisitor,
        visitor_v: &mut impl SendVisitor,
    ) -> anyhow::Result<()>;
}
```

Iterates over key/value pairs in a map.

### EnumSend / EnumSendAccept

```rust
pub trait EnumSend {
    fn visit(&self, accept: &mut impl EnumSendAccept) -> anyhow::Result<()>;
}

pub trait EnumSendAccept {
    fn accept_unit(&mut self, index: isize) -> anyhow::Result<()>;
    fn accept_newtype(&mut self, index: isize, visitor: &impl NewTypeSend) -> anyhow::Result<()>;
    fn accept_tuple(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()>;
    fn accept_named(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()>;
}
```

`EnumSend::visit` calls the appropriate `accept_*` method on the acceptor
based on the current variant. The `index` is the explicit variant id.

Schema-specific recv traits
----------------------------

### PrimitiveRecv

```rust
pub trait PrimitiveRecv {
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}
```

### OptionRecv

```rust
pub trait OptionRecv {
    fn as_some(&mut self, some: bool) -> anyhow::Result<()>;
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}
```

`as_some(true)` signals a present value; `visit()` receives the inner value.

### NewTypeRecv

```rust
pub trait NewTypeRecv {
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}
```

### SequenceRecv

```rust
pub trait SequenceRecv {
    fn len(&mut self, size: Option<usize>) -> anyhow::Result<()>;
    fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}
```

`len()` sets the expected number of elements; `visit_next()` receives each.

### TupleRecv

```rust
pub trait TupleRecv {
    fn visit(&mut self, index: isize, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}
```

### MapRecv

```rust
pub trait MapRecv {
    fn len(&mut self, size: Option<usize>) -> anyhow::Result<()>;
    fn visit_next(
        &mut self,
        visitor_k: &mut impl RecvVisitor,
        visitor_v: &mut impl RecvVisitor,
    ) -> anyhow::Result<()>;
}
```

### EnumRecv / EnumRecvAccept

```rust
pub trait EnumRecv {
    fn visit(&mut self, index: isize, accept: &mut impl EnumRecvAccept) -> anyhow::Result<()>;
}

pub trait EnumRecvAccept {
    fn accept_newtype(&mut self, visitor: &mut impl NewTypeRecv) -> anyhow::Result<()>;
    fn accept_tuple(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()>;
    fn accept_named(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()>;
}
```

`EnumRecv::visit` receives the variant index, then the acceptor dispatches
to the appropriate recv trait for the variant's payload.

Acceptor traits
---------------

### SendAccept

```rust
pub trait SendAccept {
    fn accept_primitive(&mut self, send: &impl PrimitiveSend) -> anyhow::Result<()>;
    fn accept_option(&mut self, send: &impl OptionSend) -> anyhow::Result<()>;
    fn accept_sequence(&mut self, send: &impl SequenceSend) -> anyhow::Result<()>;
    fn accept_tuple(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;
    fn accept_map(&mut self, send: &impl MapSend) -> anyhow::Result<()>;
    fn accept_newtype_struct(&mut self, send: &impl NewTypeSend) -> anyhow::Result<()>;
    fn accept_tuple_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;
    fn accept_named_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;
    fn accept_enum(&mut self, send: &impl EnumSend) -> anyhow::Result<()>;
}
```

### RecvAccept

```rust
pub trait RecvAccept {
    fn accept_primitive(&mut self, recv: &mut impl PrimitiveRecv) -> anyhow::Result<()>;
    fn accept_option(&mut self, recv: &mut impl OptionRecv) -> anyhow::Result<()>;
    fn accept_sequence(&mut self, recv: &mut impl SequenceRecv) -> anyhow::Result<()>;
    fn accept_tuple(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;
    fn accept_map(&mut self, recv: &mut impl MapRecv) -> anyhow::Result<()>;
    fn accept_newtype_struct(&mut self, recv: &mut impl NewTypeRecv) -> anyhow::Result<()>;
    fn accept_tuple_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;
    fn accept_named_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;
    fn accept_enum(&mut self, recv: &mut impl EnumRecv) -> anyhow::Result<()>;
}
```

Transport implementations receive these acceptor traits and must dispatch to
the appropriate schema-specific trait based on the wire data or schema.

TypeShape trait
---------------

```rust
pub trait TypeShape: Sized + Send + 'static {
    const SCHEMA: &'static Schema<'static>;

    fn send(&self, version: Version, send: &mut impl SendAccept) -> anyhow::Result<()>;
    fn recv(version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self>;
}
```

Every type that participates in the kaloron protocol implements `TypeShape`.
The `SCHEMA` constant describes the type's structure. `send` and `recv`
accept a protocol `Version` parameter which controls version-gated behavior.

### Builtin implementations

Builtin `TypeShape` impls are provided for:

- **Primitives**: `bool`, `i8`–`i128`, `u8`–`u128`, `f32`, `f64`, `char`,
  `String`, `Box<[u8]>`, and on Unix-like targets `std::os::fd::OwnedFd` — use
  `PrimitiveSend`/`PrimitiveRecv`.
- **Option<T>**: uses `OptionSend`/`OptionRecv`.
- **Sequences**: `Vec<T>`, `VecDeque<T>`, `LinkedList<T>`, `HashSet<T>`,
  `BTreeSet<T>` — use `SequenceSend`/`SequenceRecv`.
- **Maps**: `HashMap<K,V>`, `BTreeMap<K,V>` — use `MapSend`/`MapRecv`.
- **Tuples**: 1–32 element tuples — use `TupleSend`/`TupleRecv`.

### Derive macro implementations

For user-defined types, `#[derive(TypeShape)]` generates:

- **Unit structs**: `accept_tuple_struct` with zero fields.
- **Newtype structs**: `accept_newtype_struct` via `__Tv01`/`__Tb01`.
- **Tuple structs**: `accept_tuple_struct` via the `__Tv`/`__Tb` tree.
- **Named structs**: `accept_named_struct` via the `__Tv`/`__Tb` tree.
  Fields are visited in explicit id order.
- **Enums**: `accept_enum` with variant dispatch by explicit id.

Utility types (kaloron/src/utility.rs)
--------------------------------------

The crate provides internal utility types that form a tree-structured builder
system for handling structs and tuples of any size (up to 32 fields per chunk,
composable to arbitrary depths).

### Send adapters (`__Tv` family)

- `__Tv0` — zero-element send adapter
- `__Tv01`..`__Tv16` — 1–16 element send adapters (borrowed references)
- `__TvCp` — composite send adapter grouping up to 16 child `__Tv` nodes

Each implements `TupleSend` and maps an integer index to the corresponding
field reference. For named structs, generated code binds each field as
`field_N = &self.field_name` and constructs the `__Tv` tree.

### Recv builders (`__Tb` family)

- `__Tb0` — zero-element recv builder
- `__Tb01`..`__Tb32` — 1–32 element recv builders (optional storage)
- `__TbCp` — composite recv builder grouping up to 16 child `__Tb` nodes

Each implements `TupleRecv` and provides:
- `s{N}(value)` — set the Nth field
- `g{N}() → T` — take ownership of the Nth field
- `h{N}() → bool` — check if the Nth field is set

### Completeness checking

`__tbc(builder)` verifies all fields in a builder have been filled.

### Activation checking

- `__tvac(send_adapter, activation_states)` — validates that gated send values
  match their activation state (active values must be present for active fields,
  void for inactive fields).
- `__tbac(recv_builder, activation_states)` — validates that received gated
  values match their activation state.

Version integration in send/recv
---------------------------------

### Send flow

1. Generated `send` method receives `version: Version`.
2. For named structs with gated fields:
   - Look up activation states via `SCHEMA.as_named().active_fields_at(version)`.
   - Call `__tvac` to validate gated field values match activation state.
3. Forward to `accept_named_struct`.

### Recv flow

1. Generated `recv` method receives `version: Version`.
2. Receive all fields via `accept_named_struct`.
3. For named structs with gated fields:
   - Look up activation states via `SCHEMA.as_named().active_fields_at(version)`.
   - Call `__tbac` to validate/fixup gated field values.
4. Run completeness check with `__tbc`.

### Enum send/recv

1. **Send**: Look up variant activation states. Validate current variant is
   not inactive. Then dispatch via `EnumSend::visit` with the variant's
   explicit id as the index.
2. **Recv**: Look up variant activation states. After receiving variant index,
   validate it's not inactive. Then dispatch via `EnumRecv::visit`.

Error handling
--------------

All visitor methods return `anyhow::Result`. Common error conditions:

- **Index out of bounds**: tuple/struct field index exceeds count.
- **Incomplete builder**: not all fields were visited during recv.
- **Sequence exhausted**: `visit_next` called after all elements consumed.
- **Duplicate map key/set element**: detected during recv.
- **Version mismatch**: gated field active when it should be void, or vice versa.
- **Variant not active**: send/recv of a variant that doesn't exist at version.
