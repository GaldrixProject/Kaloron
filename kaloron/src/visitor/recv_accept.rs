// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Adapter traits and factory functions for dispatching recv operations by schema kind.
//!
//! Each `RecvAccept*` trait pairs with a `recv_accept_*` function that wraps it in a
//! `RecvAccept` impl, panicking (via `bail!`) on any non-matching schema kind.

use super::{
    EnumRecv, MapRecv, NewTypeRecv, OptionRecv, PrimitiveRecv, RecvAccept, SequenceRecv, TupleRecv,
};
use anyhow::bail;

pub trait RecvAcceptPrimitive {
    fn accept(&mut self, accept: &mut impl PrimitiveRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_primitive<T: RecvAcceptPrimitive>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptPrimitive>(T);

    impl<T: RecvAcceptPrimitive> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            RecvAcceptPrimitive::accept(&mut self.0, recv)
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptOption {
    fn accept(&mut self, accept: &mut impl OptionRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_option<T: RecvAcceptOption>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptOption>(T);

    impl<T: RecvAcceptOption> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            RecvAcceptOption::accept(&mut self.0, recv)
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptSequence {
    fn accept(&mut self, accept: &mut impl SequenceRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_sequence<T: RecvAcceptSequence>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptSequence>(T);

    impl<T: RecvAcceptSequence> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            RecvAcceptSequence::accept(&mut self.0, recv)
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptTuple {
    fn accept(&mut self, accept: &mut impl TupleRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_tuple<T: RecvAcceptTuple>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptTuple>(T);

    impl<T: RecvAcceptTuple> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            RecvAcceptTuple::accept(&mut self.0, recv)
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptMap {
    fn accept(&mut self, accept: &mut impl MapRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_map<T: RecvAcceptMap>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptMap>(T);

    impl<T: RecvAcceptMap> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, recv: &mut impl MapRecv) -> anyhow::Result<()> {
            RecvAcceptMap::accept(&mut self.0, recv)
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptNewTypeStruct {
    fn accept(&mut self, accept: &mut impl NewTypeRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_newtype_struct<T: RecvAcceptNewTypeStruct>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptNewTypeStruct>(T);

    impl<T: RecvAcceptNewTypeStruct> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            RecvAcceptNewTypeStruct::accept(&mut self.0, recv)
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptTupleStruct {
    fn accept(&mut self, accept: &mut impl TupleRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_tuple_struct<T: RecvAcceptTupleStruct>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptTupleStruct>(T);

    impl<T: RecvAcceptTupleStruct> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            RecvAcceptTupleStruct::accept(&mut self.0, recv)
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptNamedStruct {
    fn accept(&mut self, accept: &mut impl TupleRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_named_struct<T: RecvAcceptNamedStruct>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptNamedStruct>(T);

    impl<T: RecvAcceptNamedStruct> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            RecvAcceptNamedStruct::accept(&mut self.0, recv)
        }
        fn accept_enum(&mut self, _recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            bail!("unexpected enum during decoding")
        }
    }
    Delegate(v)
}

pub trait RecvAcceptEnum {
    fn accept(&mut self, accept: &mut impl EnumRecv) -> anyhow::Result<()>;
}

pub fn recv_accept_enum<T: RecvAcceptEnum>(v: T) -> impl RecvAccept {
    struct Delegate<T: RecvAcceptEnum>(T);

    impl<T: RecvAcceptEnum> RecvAccept for Delegate<T> {
        fn accept_primitive(&mut self, _recv: &mut impl PrimitiveRecv) -> anyhow::Result<()> {
            bail!("unexpected primitive during decoding")
        }
        fn accept_option(&mut self, _recv: &mut impl OptionRecv) -> anyhow::Result<()> {
            bail!("unexpected option during decoding")
        }
        fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> anyhow::Result<()> {
            bail!("unexpected sequence during decoding")
        }
        fn accept_tuple(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple during decoding")
        }
        fn accept_map(&mut self, _recv: &mut impl MapRecv) -> anyhow::Result<()> {
            bail!("unexpected map during decoding")
        }
        fn accept_newtype_struct(&mut self, _recv: &mut impl NewTypeRecv) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during decoding")
        }
        fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during decoding")
        }
        fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> anyhow::Result<()> {
            bail!("unexpected named struct during decoding")
        }
        fn accept_enum(&mut self, recv: &mut impl EnumRecv) -> anyhow::Result<()> {
            RecvAcceptEnum::accept(&mut self.0, recv)
        }
    }
    Delegate(v)
}
