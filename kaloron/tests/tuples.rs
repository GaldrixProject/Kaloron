// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use anyhow::{Result, anyhow};
use kaloron::{
    Primitive, RecvAccept, RecvVisitor, Schema, SendAccept, SendVisitor, TupleRecv, TupleSend,
    TypeShape, Version,
};
use std::collections::VecDeque;

macro_rules! tuple32 {
    ($($value:expr),+ $(,)?) => {
        ($($value,)+)
    };
}

type Tuple32 = (
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
);

struct MockNoopSendVisitor;

impl SendVisitor for MockNoopSendVisitor {
    fn visit<T: TypeShape>(&mut self, _value: &T) -> Result<()> {
        Ok(())
    }
}

struct MockTupleRecorder {
    schema: &'static Schema<'static>,
    entries: Vec<usize>,
}

impl MockTupleRecorder {
    fn new(schema: &'static Schema<'static>) -> Self {
        Self {
            schema,
            entries: Vec::new(),
        }
    }
}

impl SendAccept for MockTupleRecorder {
    fn accept_primitive(&mut self, _send: &impl kaloron::PrimitiveSend) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _send: &impl kaloron::SequenceSend) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, send: &impl TupleSend) -> Result<()> {
        let len = match self.schema {
            Schema::Tuple(elems) => elems.len(),
            other => panic!("expected tuple schema, got {other:?}"),
        };

        for index in 0..len {
            let mut visitor = MockNoopSendVisitor;
            send.visit(index as isize, &mut visitor)?;
            self.entries.push(index);
        }

        Ok(())
    }
    fn accept_map(&mut self, _send: &impl kaloron::MapSend) -> Result<()> {
        Err(anyhow!("unexpected map"))
    }
    fn accept_newtype_struct(&mut self, _send: &impl kaloron::NewTypeSend) -> Result<()> {
        Err(anyhow!("unexpected newtype struct"))
    }
    fn accept_tuple_struct(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
        Err(anyhow!("unexpected tuple struct"))
    }
    fn accept_named_struct(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
        Err(anyhow!("unexpected struct"))
    }
    fn accept_enum(&mut self, _send: &impl kaloron::EnumSend) -> Result<()> {
        Err(anyhow!("unexpected enum"))
    }
}

struct MockQueueRecvVisitor<'a> {
    values: &'a mut VecDeque<u8>,
}

impl RecvVisitor for MockQueueRecvVisitor<'_> {
    fn visit<T: TypeShape>(&mut self) -> Result<T> {
        fn cast<T, U>(value: U) -> T {
            let value = std::mem::ManuallyDrop::new(value);
            unsafe { std::ptr::read((&*value as *const U).cast::<T>()) }
        }

        let value = self
            .values
            .pop_front()
            .ok_or_else(|| anyhow!("missing recv value"))?;

        if std::any::type_name::<T>() == std::any::type_name::<u8>() {
            return Ok(cast::<T, u8>(value));
        }

        Err(anyhow!("unsupported recv type request"))
    }
}

struct MockTupleLoader {
    schema: &'static Schema<'static>,
    values: VecDeque<u8>,
}

impl MockTupleLoader {
    fn new(schema: &'static Schema<'static>, values: impl IntoIterator<Item = u8>) -> Self {
        Self {
            schema,
            values: values.into_iter().collect(),
        }
    }
}

impl RecvAccept for MockTupleLoader {
    fn accept_primitive(&mut self, _recv: &mut impl kaloron::PrimitiveRecv) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _recv: &mut impl kaloron::OptionRecv) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _recv: &mut impl kaloron::SequenceRecv) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, recv: &mut impl TupleRecv) -> Result<()> {
        let len = match self.schema {
            Schema::Tuple(elems) => elems.len(),
            other => panic!("expected tuple schema, got {other:?}"),
        };

        for index in 0..len {
            let mut visitor = MockQueueRecvVisitor {
                values: &mut self.values,
            };
            recv.visit(index as isize, &mut visitor)?;
        }

        Ok(())
    }
    fn accept_map(&mut self, _recv: &mut impl kaloron::MapRecv) -> Result<()> {
        Err(anyhow!("unexpected map"))
    }
    fn accept_newtype_struct(&mut self, _recv: &mut impl kaloron::NewTypeRecv) -> Result<()> {
        Err(anyhow!("unexpected newtype struct"))
    }
    fn accept_tuple_struct(&mut self, _recv: &mut impl TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple struct"))
    }
    fn accept_named_struct(&mut self, _recv: &mut impl TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected struct"))
    }
    fn accept_enum(&mut self, _recv: &mut impl kaloron::EnumRecv) -> Result<()> {
        Err(anyhow!("unexpected enum"))
    }
}

#[test]
fn test_tuple_schema_supports_32_elements() {
    let schema = <Tuple32 as TypeShape>::SCHEMA;
    let elems = match schema {
        Schema::Tuple(elems) => elems,
        other => panic!("expected tuple schema, got {other:?}"),
    };

    assert_eq!(elems.len(), 32);
    assert!(
        elems
            .iter()
            .all(|ty| **ty == Schema::Primitive(Primitive::U8))
    );
}

#[test]
fn test_tuple_send_visits_all_32_elements_in_order() {
    let value: Tuple32 = tuple32!(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    );
    let mut recorder = MockTupleRecorder::new(<Tuple32 as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.entries, (0..32).collect::<Vec<_>>());
}

#[test]
fn test_tuple_recv_populates_all_32_elements_in_order() {
    let mut loader = MockTupleLoader::new(<Tuple32 as TypeShape>::SCHEMA, 0u8..32u8);

    let value: Tuple32 = <Tuple32 as TypeShape>::recv(Version::zero(), &mut loader).unwrap();

    let (
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        a8,
        a9,
        a10,
        a11,
        a12,
        a13,
        a14,
        a15,
        a16,
        a17,
        a18,
        a19,
        a20,
        a21,
        a22,
        a23,
        a24,
        a25,
        a26,
        a27,
        a28,
        a29,
        a30,
        a31,
    ) = value;

    assert_eq!(
        [
            a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18,
            a19, a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31,
        ],
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]
    );
}

#[test]
fn test_tuple_visit_rejects_out_of_bounds_indices() {
    struct MockOutOfBoundsRecorder;

    impl SendAccept for MockOutOfBoundsRecorder {
        fn accept_primitive(&mut self, _send: &impl kaloron::PrimitiveSend) -> Result<()> {
            Err(anyhow!("unexpected primitive"))
        }
        fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> Result<()> {
            Err(anyhow!("unexpected option"))
        }
        fn accept_sequence(&mut self, _send: &impl kaloron::SequenceSend) -> Result<()> {
            Err(anyhow!("unexpected sequence"))
        }
        fn accept_tuple(&mut self, send: &impl TupleSend) -> Result<()> {
            let mut visitor = MockNoopSendVisitor;
            send.visit(32, &mut visitor)
        }
        fn accept_map(&mut self, _send: &impl kaloron::MapSend) -> Result<()> {
            Err(anyhow!("unexpected map"))
        }
        fn accept_newtype_struct(&mut self, _send: &impl kaloron::NewTypeSend) -> Result<()> {
            Err(anyhow!("unexpected newtype struct"))
        }
        fn accept_tuple_struct(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
            Err(anyhow!("unexpected tuple struct"))
        }
        fn accept_named_struct(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
            Err(anyhow!("unexpected struct"))
        }
        fn accept_enum(&mut self, _send: &impl kaloron::EnumSend) -> Result<()> {
            Err(anyhow!("unexpected enum"))
        }
    }

    let value: Tuple32 = tuple32!(
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    );

    let err = value
        .send(Version::zero(), &mut MockOutOfBoundsRecorder)
        .unwrap_err();
    assert_eq!(err.to_string(), "tuple index out of bounds");
}
