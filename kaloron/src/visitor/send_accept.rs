// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Adapter traits and factory functions for dispatching send operations by schema kind.
//!
//! Each `SendAccept*` trait pairs with a `send_accept_*` function that wraps it in a
//! `SendAccept` impl, panicking (via `bail!`) on any non-matching schema kind.

use super::{
    EnumSend, MapSend, NewTypeSend, OptionSend, PrimitiveSend, SendAccept, SequenceSend, TupleSend,
};
use anyhow::bail;

pub trait SendAcceptPrimitive {
    fn accept(&mut self, accept: &impl PrimitiveSend) -> anyhow::Result<()>;
}

pub fn send_accept_primitive<T: SendAcceptPrimitive>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptPrimitive>(T);

    impl<T: SendAcceptPrimitive> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, send: &impl PrimitiveSend) -> anyhow::Result<()> {
            SendAcceptPrimitive::accept(&mut self.0, send)
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptOption {
    fn accept(&mut self, accept: &impl OptionSend) -> anyhow::Result<()>;
}

pub fn send_accept_option<T: SendAcceptOption>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptOption>(T);

    impl<T: SendAcceptOption> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, send: &impl OptionSend) -> anyhow::Result<()> {
            SendAcceptOption::accept(&mut self.0, send)
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptSequence {
    fn accept(&mut self, accept: &impl SequenceSend) -> anyhow::Result<()>;
}

pub fn send_accept_sequence<T: SendAcceptSequence>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptSequence>(T);

    impl<T: SendAcceptSequence> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, send: &impl SequenceSend) -> anyhow::Result<()> {
            SendAcceptSequence::accept(&mut self.0, send)
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptTuple {
    fn accept(&mut self, accept: &impl TupleSend) -> anyhow::Result<()>;
}

pub fn send_accept_tuple<T: SendAcceptTuple>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptTuple>(T);

    impl<T: SendAcceptTuple> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            SendAcceptTuple::accept(&mut self.0, send)
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptMap {
    fn accept(&mut self, accept: &impl MapSend) -> anyhow::Result<()>;
}

pub fn send_accept_map<T: SendAcceptMap>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptMap>(T);

    impl<T: SendAcceptMap> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, send: &impl MapSend) -> anyhow::Result<()> {
            SendAcceptMap::accept(&mut self.0, send)
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptNewTypeStruct {
    fn accept(&mut self, accept: &impl NewTypeSend) -> anyhow::Result<()>;
}

pub fn send_accept_newtype_struct<T: SendAcceptNewTypeStruct>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptNewTypeStruct>(T);

    impl<T: SendAcceptNewTypeStruct> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, send: &impl NewTypeSend) -> anyhow::Result<()> {
            SendAcceptNewTypeStruct::accept(&mut self.0, send)
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptTupleStruct {
    fn accept(&mut self, accept: &impl TupleSend) -> anyhow::Result<()>;
}

pub fn send_accept_tuple_struct<T: SendAcceptTupleStruct>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptTupleStruct>(T);

    impl<T: SendAcceptTupleStruct> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            SendAcceptTupleStruct::accept(&mut self.0, send)
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptNamedStruct {
    fn accept(&mut self, accept: &impl TupleSend) -> anyhow::Result<()>;
}

pub fn send_accept_named_struct<T: SendAcceptNamedStruct>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptNamedStruct>(T);

    impl<T: SendAcceptNamedStruct> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, send: &impl TupleSend) -> anyhow::Result<()> {
            SendAcceptNamedStruct::accept(&mut self.0, send)
        }
        fn accept_enum(&mut self, _send: &impl EnumSend) -> anyhow::Result<()> {
            bail!("unexpected enum during encoding")
        }
    }
    Delegate(v)
}

pub trait SendAcceptEnum {
    fn accept(&mut self, accept: &impl EnumSend) -> anyhow::Result<()>;
}

pub fn send_accept_enum<T: SendAcceptEnum>(v: T) -> impl SendAccept {
    struct Delegate<T: SendAcceptEnum>(T);

    impl<T: SendAcceptEnum> SendAccept for Delegate<T> {
        fn accept_primitive(&mut self, _send: &impl PrimitiveSend) -> anyhow::Result<()> {
            bail!("unexpected primitive during encoding")
        }
        fn accept_option(&mut self, _send: &impl OptionSend) -> anyhow::Result<()> {
            bail!("unexpected option during encoding")
        }
        fn accept_sequence(&mut self, _send: &impl SequenceSend) -> anyhow::Result<()> {
            bail!("unexpected sequence during encoding")
        }
        fn accept_tuple(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple during encoding")
        }
        fn accept_map(&mut self, _send: &impl MapSend) -> anyhow::Result<()> {
            bail!("unexpected map during encoding")
        }
        fn accept_newtype_struct(&mut self, _send: &impl NewTypeSend) -> anyhow::Result<()> {
            bail!("unexpected newtype struct during encoding")
        }
        fn accept_tuple_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected tuple struct during encoding")
        }
        fn accept_named_struct(&mut self, _send: &impl TupleSend) -> anyhow::Result<()> {
            bail!("unexpected named struct during encoding")
        }
        fn accept_enum(&mut self, send: &impl EnumSend) -> anyhow::Result<()> {
            SendAcceptEnum::accept(&mut self.0, send)
        }
    }
    Delegate(v)
}
