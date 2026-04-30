// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Core visitor traits and value transport surfaces used by the kaloron
//! serialization/deserialization runtime.
//!
//! The crate uses a small visitor protocol to walk values according to a
//! `Schema`. Each `ValueSend`/`ValueRecv` adapter accepts a visitor object
//! that implements the appropriate `*Send`/`*Recv` traits for the schema
//! shape it represents.

use crate::{Schema, Version};

mod recv_accept;
mod send_accept;

pub use recv_accept::*;
pub use send_accept::*;

/// Generic visitor used to inspect a single nested value when sending.
///
/// Implementors provide a generic `visit` method that is called with a
/// shared reference to a nested value.
///
/// Generic parameters:
/// - `T`: the concrete type requested by the callee. Implementations should
///   handle the actual type cast/dispatch as needed (usually via `Any` or
///   type-erased strategies inside the runtime tests).
pub trait SendVisitor {
    /// Visit a particular value by shared reference.
    ///
    /// Arguments:
    /// - `value`: a shared reference to the value to be visited.
    ///
    /// Returns:
    /// - `Ok(())` when the visitor successfully handled the value.
    /// - An `Err` if the visitor cannot handle the requested value.
    fn visit<T: TypeShape>(&mut self, value: &T) -> anyhow::Result<()>;
}

/// Generic visitor used to receive a single nested value when receiving.
///
/// Implementors return an owned `T` value from `visit`.
///
/// Generic parameters:
/// - `T`: the type to be produced by the visitor.
pub trait RecvVisitor {
    /// Receive the next value as `T`.
    ///
    /// Returns:
    /// - `Ok(T)` containing the produced value on success.
    /// - `Err(...)` if the value could not be constructed.
    fn visit<T: TypeShape>(&mut self) -> anyhow::Result<T>;
}

/// Visitor surface for send values with primitive schema type (e.g. integers,
/// strings).
///
/// Implementors provide the logic to forward the primitive value to a
/// `SendVisitor` by shared reference.
pub trait PrimitiveSendVisitor {
    fn visit_bool(&mut self, v: &bool) -> anyhow::Result<()>;
    fn visit_i8(&mut self, v: &i8) -> anyhow::Result<()>;
    fn visit_i16(&mut self, v: &i16) -> anyhow::Result<()>;
    fn visit_i32(&mut self, v: &i32) -> anyhow::Result<()>;
    fn visit_i64(&mut self, v: &i64) -> anyhow::Result<()>;
    fn visit_i128(&mut self, v: &i128) -> anyhow::Result<()>;
    fn visit_u8(&mut self, v: &u8) -> anyhow::Result<()>;
    fn visit_u16(&mut self, v: &u16) -> anyhow::Result<()>;
    fn visit_u32(&mut self, v: &u32) -> anyhow::Result<()>;
    fn visit_u64(&mut self, v: &u64) -> anyhow::Result<()>;
    fn visit_u128(&mut self, v: &u128) -> anyhow::Result<()>;
    fn visit_f32(&mut self, v: &f32) -> anyhow::Result<()>;
    fn visit_f64(&mut self, v: &f64) -> anyhow::Result<()>;
    fn visit_char(&mut self, v: &char) -> anyhow::Result<()>;
    #[allow(clippy::ptr_arg)]
    fn visit_string(&mut self, v: &String) -> anyhow::Result<()>;

    #[cfg(unix)]
    fn visit_file(&mut self, v: &std::os::fd::OwnedFd) -> anyhow::Result<()>;
}

pub trait PrimitiveRecvVisitor {
    fn visit_bool(&mut self) -> anyhow::Result<bool>;
    fn visit_i8(&mut self) -> anyhow::Result<i8>;
    fn visit_i16(&mut self) -> anyhow::Result<i16>;
    fn visit_i32(&mut self) -> anyhow::Result<i32>;
    fn visit_i64(&mut self) -> anyhow::Result<i64>;
    fn visit_i128(&mut self) -> anyhow::Result<i128>;
    fn visit_u8(&mut self) -> anyhow::Result<u8>;
    fn visit_u16(&mut self) -> anyhow::Result<u16>;
    fn visit_u32(&mut self) -> anyhow::Result<u32>;
    fn visit_u64(&mut self) -> anyhow::Result<u64>;
    fn visit_u128(&mut self) -> anyhow::Result<u128>;
    fn visit_f32(&mut self) -> anyhow::Result<f32>;
    fn visit_f64(&mut self) -> anyhow::Result<f64>;
    fn visit_char(&mut self) -> anyhow::Result<char>;
    fn visit_string(&mut self) -> anyhow::Result<String>;

    #[cfg(unix)]
    fn visit_file(&mut self) -> anyhow::Result<std::os::fd::OwnedFd>;
}

pub trait PrimitiveSend {
    /// Visit the primitive value.
    ///
    /// Arguments:
    /// - `visitor`: the send visitor that will be called with a reference to
    ///   the primitive.
    ///
    /// Returns:
    /// - `Ok(())` when the visit completes successfully.
    /// - `Err(...)` when the visitor fails.
    fn visit(&self, visitor: &mut impl PrimitiveSendVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for receiving a primitive value.
///
/// Implementors should call the provided `RecvVisitor` to obtain the primitive
/// value and store it into the receiving structure.
pub trait PrimitiveRecv {
    /// Receive the primitive value.
    ///
    /// Arguments:
    /// - `visitor`: the receive visitor used to construct the primitive value.
    ///
    /// Returns:
    /// - `Ok(())` when the primitive has been successfully received.
    /// - `Err(...)` if the visitor fails or the value cannot be received.
    fn visit(&mut self, visitor: &mut impl PrimitiveRecvVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Option` send values.
///
/// Allows the caller to query whether the option is currently `Some` and, if
/// so, visit the contained value.
pub trait OptionSend {
    /// Returns true if the option currently contains a value.
    fn is_some(&self) -> bool;

    /// Visit the contained value when this option is `Some`.
    ///
    /// Arguments:
    /// - `visitor`: the `SendVisitor` that will receive the inner value.
    ///
    /// Returns:
    /// - `Ok(())` when the inner value was successfully visited.
    /// - `Err(...)` if the visit fails.
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Option` recv values.
///
/// `as_some` signals whether the option will contain a value. When
/// `as_some(true)` is called, `visit` will be invoked to receive the inner
/// value.
pub trait OptionRecv {
    /// Set whether the option should contain a value.
    ///
    /// Arguments:
    /// - `some`: `true` if the option will contain a value, `false` otherwise.
    ///
    /// Returns:
    /// - `Ok(())` on success or `Err(...)` if the adapter cannot handle the
    ///   `as_some` signal.
    fn as_some(&mut self, some: bool) -> anyhow::Result<()>;

    /// Receive the contained value when this option is `Some`.
    ///
    /// Arguments:
    /// - `visitor`: the `RecvVisitor` to build the contained value.
    ///
    /// Returns:
    /// - `Ok(())` when the inner value was successfully received.
    /// - `Err(...)` if receiving the inner value failed.
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::NewTypeStruct` send values.
///
/// Used for tuple-like structs that wrap a single value.
pub trait NewTypeSend {
    /// Visit the wrapped value.
    ///
    /// Arguments:
    /// - `visitor`: the `SendVisitor` that will receive the inner value.
    ///
    /// Returns: `Ok(())` on success or `Err(...)` on failure.
    fn visit(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::NewTypeStruct` recv values.
pub trait NewTypeRecv {
    /// Receive the wrapped value.
    ///
    /// Arguments:
    /// - `visitor`: the `RecvVisitor` used to build the wrapped value.
    ///
    /// Returns: `Ok(())` on success or `Err(...)` on failure.
    fn visit(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Seq` send values.
pub trait SequenceSend {
    /// Number of elements currently present in the sequence, if known.
    fn len(&self) -> Option<usize>;

    /// Returns true if a call to `visit_next` will yield an element without
    /// producing an error. This lets callers detect the end of sequences when
    /// the length is not known ahead of time.
    fn has_next(&self) -> bool;

    /// Returns true if the sequence is empty. Default implementation uses
    /// `len` when available or `has_next` when length is unknown.
    fn is_empty(&self) -> bool {
        match self.len() {
            Some(n) => n == 0,
            None => !self.has_next(),
        }
    }

    /// Visit the next element in the sequence.
    ///
    /// Arguments:
    /// - `visitor`: the `SendVisitor` that will receive the element.
    fn visit_next(&self, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Seq` recv values.
pub trait SequenceRecv {
    /// Set the (optional) number of elements that will be received.
    /// This function must be called before any visit_next is called, or the behavior is error.
    /// This function must be called only once per accepted, otherwise is error.
    ///
    /// Arguments:
    /// - `size`: the number of elements that will be sent by the caller, or
    ///   `None` when the sender streams items without a known length ahead of
    ///   time.
    fn len(&mut self, size: Option<usize>) -> anyhow::Result<()>;

    /// Receive the next element.
    ///
    /// Arguments:
    /// - `visitor`: the `RecvVisitor` used to build the next element.
    fn visit_next(&mut self, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for positional send values used by `Schema::Named`,
/// `Schema::TupleStruct`, and `Schema::Tuple`.
///
/// Implementors are responsible for visiting each element by index and
/// forwarding it to the `SendVisitor`.
pub trait TupleSend {
    /// Visit an element by its index.
    ///
    /// Arguments:
    /// - `index`: the zero-based index of the field or element in the schema.
    /// - `visitor`: the `SendVisitor` used to visit the field or element.
    ///
    /// Returns:
    /// - `Ok(())` on success, or `Err(...)` when the visit fails.
    fn visit(&self, index: isize, visitor: &mut impl SendVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for index-based recv values used by structs, tuple
/// structs, and tuples.
///
/// Implementors receive fields/elements by index and should construct the
/// target value using the provided `RecvVisitor`.
pub trait TupleRecv {
    /// Receive an element by its index.
    ///
    /// Arguments:
    /// - `index`: zero-based element/field index.
    /// - `visitor`: `RecvVisitor` to build the value.
    fn visit(&mut self, index: isize, visitor: &mut impl RecvVisitor) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Map` send values.
pub trait MapSend {
    /// Number of entries currently present in the map, if known.
    fn len(&self) -> Option<usize>;

    /// Returns true if a call to `visit_next` will yield an entry without
    /// producing an error. This lets callers detect the end of maps when the
    /// length is not known ahead of time.
    fn has_next(&self) -> bool;

    /// Returns true if the map is empty. Default implementation uses `len`
    /// when available or `has_next` when length is unknown.
    fn is_empty(&self) -> bool {
        match self.len() {
            Some(n) => n == 0,
            None => !self.has_next(),
        }
    }

    /// Visit the next key/value entry.
    ///
    /// Arguments:
    /// - `visitor_k`: visitor for the key of the next entry.
    /// - `visitor_v`: visitor for the value of the next entry.
    fn visit_next(
        &self,
        visitor_k: &mut impl SendVisitor,
        visitor_v: &mut impl SendVisitor,
    ) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Map` recv values.
pub trait MapRecv {
    /// Set the number of entries that will be received.
    ///
    /// Arguments:
    /// - `size`: number of entries to expect.
    fn len(&mut self, size: usize) -> anyhow::Result<()>;

    /// Receive the next key/value entry.
    ///
    /// Arguments:
    /// - `visitor_k`: `RecvVisitor` used to build the next key.
    /// - `visitor_v`: `RecvVisitor` used to build the next value.
    fn visit_next(
        &mut self,
        visitor_k: &mut impl RecvVisitor,
        visitor_v: &mut impl RecvVisitor,
    ) -> anyhow::Result<()>;
}

pub trait EnumSendAccept {
    /// Receive the payload of a `VariantKind::Unit` variant.
    ///
    /// Arguments:
    /// - `index`: variant index as defined by the enum schema.
    fn accept_unit(&mut self, index: isize) -> anyhow::Result<()>;
    /// Receive the payload of a `VariantKind::NewType` variant.
    ///
    /// Arguments:
    /// - `index`: variant index as defined by the enum schema.
    /// - `visitor`: the `SendVisitor` used to inspect the newtype payload.
    fn accept_newtype(&mut self, index: isize, visitor: &impl NewTypeSend) -> anyhow::Result<()>;

    /// Receive an element of a `VariantKind::Tuple` variant by index.
    ///
    /// Arguments:
    /// - `index`: variant index as defined by the enum schema.
    /// - `visitor`: the `SendVisitor` used to inspect the element.
    fn accept_tuple(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()>;

    /// Receive a field of a `VariantKind::Struct` variant by schema index.
    ///
    /// Arguments:
    /// - `index`: variant index as defined by the enum schema.
    /// - `visitor`: the `SendVisitor` used to inspect the field value.
    fn accept_named(&mut self, index: isize, visitor: &impl TupleSend) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Enum` send values.
pub trait EnumSend {
    /// visit to set the current active variant and its value
    ///
    /// Arguments:
    /// - `accept`: the value visitor acceptor for the enum active value.
    fn visit(&self, accept: &mut impl EnumSendAccept) -> anyhow::Result<()>;
}

pub trait EnumRecvAccept {
    /// Receive the payload of a `VariantKind::NewType` variant.
    ///
    /// Arguments:
    /// - `visitor`: the `RecvVisitor` used to build the newtype payload.
    fn accept_newtype(&mut self, visitor: &mut impl NewTypeRecv) -> anyhow::Result<()>;

    /// Receive an element of a `VariantKind::Tuple` variant by index.
    ///
    /// Arguments:
    /// - `index`: zero-based element index inside the tuple variant.
    /// - `visitor`: the `RecvVisitor` used to build the element.
    fn accept_tuple(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()>;

    /// Receive a field of a `VariantKind::Struct` variant by schema index.
    ///
    /// Arguments:
    /// - `index`: zero-based field index inside the variant's struct payload.
    /// - `visitor`: the `RecvVisitor` used to build the field value.
    fn accept_named(&mut self, visitor: &mut impl TupleRecv) -> anyhow::Result<()>;
}

/// Visitor surface for `Schema::Enum` recv values.
pub trait EnumRecv {
    /// visit to set the current active variant and its value
    ///
    /// Arguments:
    /// - `index`: variant index as defined by the enum schema.
    /// - `accept`: the value visitor acceptor for the enum active value.
    fn visit(&mut self, index: isize, accept: &mut impl EnumRecvAccept) -> anyhow::Result<()>;
}

/// Adapter that accepts send visitors for all schema kinds.
///
/// Implementors are responsible for delegating to the appropriate `*Send`
/// visitor according to the `Schema` they represent.
pub trait SendAccept {
    /// Accept a primitive send visitor.
    fn accept_primitive(&mut self, send: &impl PrimitiveSend) -> anyhow::Result<()>;

    /// Accept an option send visitor.
    fn accept_option(&mut self, send: &impl OptionSend) -> anyhow::Result<()>;

    /// Accept a sequence send visitor.
    fn accept_sequence(&mut self, send: &impl SequenceSend) -> anyhow::Result<()>;

    /// Accept a tuple send visitor.
    fn accept_tuple(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;

    /// Accept a map send visitor.
    fn accept_map(&mut self, send: &impl MapSend) -> anyhow::Result<()>;

    /// Accept a newtype-struct send visitor.
    fn accept_newtype_struct(&mut self, send: &impl NewTypeSend) -> anyhow::Result<()>;

    /// Accept a tuple-struct send visitor.
    fn accept_tuple_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;

    /// Accept a named-struct send visitor.
    fn accept_named_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()>;

    /// Accept an enum send visitor.
    fn accept_enum(&mut self, send: &impl EnumSend) -> anyhow::Result<()>;
}

/// Adapter that accepts recv visitors for all schema kinds.
///
/// Implementors are responsible for delegating to the appropriate `*Recv`
/// visitor according to the `Schema` they represent.
pub trait RecvAccept {
    /// Accept a primitive recv visitor.
    fn accept_primitive(&mut self, recv: &mut impl PrimitiveRecv) -> anyhow::Result<()>;

    /// Accept an option recv visitor.
    fn accept_option(&mut self, recv: &mut impl OptionRecv) -> anyhow::Result<()>;

    /// Accept a sequence recv visitor.
    fn accept_sequence(&mut self, recv: &mut impl SequenceRecv) -> anyhow::Result<()>;

    /// Accept a tuple recv visitor.
    fn accept_tuple(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;

    /// Accept a map recv visitor.
    fn accept_map(&mut self, recv: &mut impl MapRecv) -> anyhow::Result<()>;

    /// Accept a newtype-struct recv visitor.
    fn accept_newtype_struct(&mut self, recv: &mut impl NewTypeRecv) -> anyhow::Result<()>;

    /// Accept a tuple-struct recv visitor.
    fn accept_tuple_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;

    /// Accept a named-struct recv visitor.
    fn accept_named_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()>;

    /// Accept an enum recv visitor.
    fn accept_enum(&mut self, recv: &mut impl EnumRecv) -> anyhow::Result<()>;
}

/// Shape trait implemented for types that can be sent/received according to
/// a `Schema`.
///
/// Implementors must be `Sized + Send + 'static`, provide a static `SCHEMA`
/// describing the type, and provide `send`/`recv` implementations that
/// describe how to serialize and deserialize the type using the visitor
/// surfaces above.
pub trait TypeShape: Sized + Send + 'static {
    /// The static schema describing the type shape.
    const SCHEMA: &'static Schema<'static>;

    /// Send the value using a `ValueSend` adapter.
    ///
    /// Arguments:
    /// - `version`: the protocol version to use for this operation.
    /// - `send`: adapter that accepts `*Send` visitors. The implementation
    ///   should call the appropriate `accept_*` method(s) on `send`.
    ///
    /// Returns: `Ok(())` on success.
    fn send(&self, version: Version, send: &mut impl SendAccept) -> anyhow::Result<()>;

    /// Receive an owned value of `Self` using a `ValueRecv` adapter.
    ///
    /// Arguments:
    /// - `version`: the protocol version to use for this operation.
    /// - `recv`: adapter that accepts `*Recv` visitors. The implementation
    ///   should call `accept_*` and use visitor callbacks to construct the
    ///   resulting value.
    ///
    /// Returns: the constructed `Self` on success.
    fn recv(version: Version, recv: &mut impl RecvAccept) -> anyhow::Result<Self>;
}
