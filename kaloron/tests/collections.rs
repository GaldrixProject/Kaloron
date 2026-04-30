// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use anyhow::{Result, anyhow};
use kaloron::{
    MapRecv, MapSend, Primitive, PrimitiveRecvVisitor, PrimitiveSendVisitor, RecvAccept,
    RecvVisitor, Schema, SendAccept, SendVisitor, SequenceRecv, SequenceSend, TypeShape, Version,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::os::fd::{AsRawFd, OwnedFd};

fn assist_cast<T, U>(value: U) -> T {
    let value = std::mem::ManuallyDrop::new(value);
    unsafe { std::ptr::read((&*value as *const U).cast::<T>()) }
}

fn assist_read_u32_ref<T>(value: &T) -> Result<u32> {
    if std::any::type_name::<T>() == std::any::type_name::<u32>() {
        unsafe { Ok(*(value as *const T).cast::<u32>()) }
    } else {
        Err(anyhow!("unsupported send type request"))
    }
}

struct MockU32VecVisitor<'a> {
    output: &'a mut Vec<u32>,
}

impl SendVisitor for MockU32VecVisitor<'_> {
    fn visit<T: TypeShape>(&mut self, value: &T) -> Result<()> {
        self.output.push(assist_read_u32_ref(value)?);
        Ok(())
    }
}

struct MockU32SlotVisitor<'a> {
    slot: &'a mut Option<u32>,
}

impl SendVisitor for MockU32SlotVisitor<'_> {
    fn visit<T: TypeShape>(&mut self, value: &T) -> Result<()> {
        *self.slot = Some(assist_read_u32_ref(value)?);
        Ok(())
    }
}

struct MockU32QueueRecvVisitor<'a> {
    values: &'a mut VecDeque<u32>,
}

impl RecvVisitor for MockU32QueueRecvVisitor<'_> {
    fn visit<T: TypeShape>(&mut self) -> Result<T> {
        let value = self
            .values
            .pop_front()
            .ok_or_else(|| anyhow!("missing recv value"))?;

        if std::any::type_name::<T>() == std::any::type_name::<u32>() {
            return Ok(assist_cast::<T, u32>(value));
        }

        Err(anyhow!("unsupported recv type request"))
    }
}

struct MockSequenceRecorder {
    schema: &'static Schema<'static>,
    values: Vec<u32>,
}

impl MockSequenceRecorder {
    fn new(schema: &'static Schema<'static>) -> Self {
        Self {
            schema,
            values: Vec::new(),
        }
    }
}

impl SendAccept for MockSequenceRecorder {
    fn accept_primitive(&mut self, _send: &impl kaloron::PrimitiveSend) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, send: &impl SequenceSend) -> Result<()> {
        match self.schema {
            Schema::Seq(_) => {}
            other => panic!("expected sequence schema, got {other:?}"),
        }

        let len = send
            .len()
            .ok_or_else(|| anyhow!("sequence length was not provided"))?;

        for _ in 0..len {
            let mut visitor = MockU32VecVisitor {
                output: &mut self.values,
            };
            send.visit_next(&mut visitor)?;
        }

        Ok(())
    }

    fn accept_tuple(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
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

struct MockSequenceLoader {
    schema: &'static Schema<'static>,
    values: VecDeque<u32>,
}

impl MockSequenceLoader {
    fn new(schema: &'static Schema<'static>, values: impl IntoIterator<Item = u32>) -> Self {
        Self {
            schema,
            values: values.into_iter().collect(),
        }
    }
}

impl RecvAccept for MockSequenceLoader {
    fn accept_primitive(&mut self, _recv: &mut impl kaloron::PrimitiveRecv) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _recv: &mut impl kaloron::OptionRecv) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, recv: &mut impl SequenceRecv) -> Result<()> {
        match self.schema {
            Schema::Seq(_) => {}
            other => panic!("expected sequence schema, got {other:?}"),
        }

        let len = self.values.len();
        recv.len(Some(len))?;

        for _ in 0..len {
            let mut visitor = MockU32QueueRecvVisitor {
                values: &mut self.values,
            };
            recv.visit_next(&mut visitor)?;
        }

        Ok(())
    }

    fn accept_tuple(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
    }
    fn accept_map(&mut self, _recv: &mut impl MapRecv) -> Result<()> {
        Err(anyhow!("unexpected map"))
    }
    fn accept_newtype_struct(&mut self, _recv: &mut impl kaloron::NewTypeRecv) -> Result<()> {
        Err(anyhow!("unexpected newtype struct"))
    }
    fn accept_tuple_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple struct"))
    }
    fn accept_named_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected struct"))
    }
    fn accept_enum(&mut self, _recv: &mut impl kaloron::EnumRecv) -> Result<()> {
        Err(anyhow!("unexpected enum"))
    }
}

struct MockMapRecorder {
    schema: &'static Schema<'static>,
    entries: Vec<(u32, u32)>,
}

impl MockMapRecorder {
    fn new(schema: &'static Schema<'static>) -> Self {
        Self {
            schema,
            entries: Vec::new(),
        }
    }
}

impl SendAccept for MockMapRecorder {
    fn accept_primitive(&mut self, _send: &impl kaloron::PrimitiveSend) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _send: &impl SequenceSend) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
    }
    fn accept_map(&mut self, send: &impl MapSend) -> Result<()> {
        match self.schema {
            Schema::Map(_, _) => {}
            other => panic!("expected map schema, got {other:?}"),
        }

        let len = send
            .len()
            .ok_or_else(|| anyhow!("map length was not provided"))?;

        for _ in 0..len {
            let mut key = None;
            let mut value = None;
            let mut key_visitor = MockU32SlotVisitor { slot: &mut key };
            let mut value_visitor = MockU32SlotVisitor { slot: &mut value };
            send.visit_next(&mut key_visitor, &mut value_visitor)?;
            self.entries.push((key.unwrap(), value.unwrap()));
        }

        Ok(())
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

struct MockMapLoader {
    schema: &'static Schema<'static>,
    keys: VecDeque<u32>,
    values: VecDeque<u32>,
}

impl MockMapLoader {
    fn new(
        schema: &'static Schema<'static>,
        entries: impl IntoIterator<Item = (u32, u32)>,
    ) -> Self {
        let mut keys = VecDeque::new();
        let mut values = VecDeque::new();
        for (key, value) in entries {
            keys.push_back(key);
            values.push_back(value);
        }
        Self {
            schema,
            keys,
            values,
        }
    }
}

impl RecvAccept for MockMapLoader {
    fn accept_primitive(&mut self, _recv: &mut impl kaloron::PrimitiveRecv) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn accept_option(&mut self, _recv: &mut impl kaloron::OptionRecv) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
    }
    fn accept_map(&mut self, recv: &mut impl MapRecv) -> Result<()> {
        match self.schema {
            Schema::Map(_, _) => {}
            other => panic!("expected map schema, got {other:?}"),
        }

        assert_eq!(self.keys.len(), self.values.len());
        let len = self.keys.len();
        recv.len(len)?;

        for _ in 0..len {
            let mut key_visitor = MockU32QueueRecvVisitor {
                values: &mut self.keys,
            };
            let mut value_visitor = MockU32QueueRecvVisitor {
                values: &mut self.values,
            };
            recv.visit_next(&mut key_visitor, &mut value_visitor)?;
        }

        Ok(())
    }

    fn accept_newtype_struct(&mut self, _recv: &mut impl kaloron::NewTypeRecv) -> Result<()> {
        Err(anyhow!("unexpected newtype struct"))
    }
    fn accept_tuple_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple struct"))
    }
    fn accept_named_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected struct"))
    }
    fn accept_enum(&mut self, _recv: &mut impl kaloron::EnumRecv) -> Result<()> {
        Err(anyhow!("unexpected enum"))
    }
}

fn assist_assert_sequence_schema(
    schema: &'static Schema<'static>,
    expected_inner: &'static Schema<'static>,
) {
    match schema {
        Schema::Seq(inner) => assert_eq!(inner, &expected_inner),
        other => panic!("expected sequence schema, got {other:?}"),
    }
}

fn assist_assert_map_schema(
    schema: &'static Schema<'static>,
    expected_key: &'static Schema<'static>,
    expected_value: &'static Schema<'static>,
) {
    match schema {
        Schema::Map(key, value) => {
            assert_eq!(key, &expected_key);
            assert_eq!(value, &expected_value);
        }
        other => panic!("expected map schema, got {other:?}"),
    }
}

#[test]
fn test_generic_sequence_container_schemas_are_sequences() {
    assist_assert_sequence_schema(
        <Vec<u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_sequence_schema(
        <VecDeque<u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_sequence_schema(
        <LinkedList<u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_sequence_schema(
        <HashSet<u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_sequence_schema(
        <BTreeSet<u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_sequence_schema(
        <Vec<u8> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U8),
    );
    assist_assert_sequence_schema(
        <Box<[u8]> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U8),
    );
    assist_assert_sequence_schema(
        <[u32; 3] as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
    );
}

#[test]
fn test_map_container_schemas_are_maps() {
    assist_assert_map_schema(
        <HashMap<u32, u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
        &Schema::Primitive(Primitive::U32),
    );
    assist_assert_map_schema(
        <BTreeMap<u32, u32> as TypeShape>::SCHEMA,
        &Schema::Primitive(Primitive::U32),
        &Schema::Primitive(Primitive::U32),
    );
}

#[test]
fn test_vec_send_and_recv_use_sequence_visitors_in_order() {
    let value = vec![1u32, 2, 3];
    let mut recorder = MockSequenceRecorder::new(<Vec<u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<Vec<u32> as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: Vec<u32> = <Vec<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, vec![4, 5, 6]);
}

#[test]
fn test_box_slice_send_and_recv_use_sequence_visitors_in_order() {
    let value: Box<[u32]> = vec![1u32, 2, 3].into_boxed_slice();
    let mut recorder = MockSequenceRecorder::new(<Box<[u32]> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<Box<[u32]> as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: Box<[u32]> =
        <Box<[u32]> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(&*decoded, &[4, 5, 6]);
}

#[test]
fn test_array_send_and_recv_use_sequence_visitors_in_order() {
    let value = [1u32, 2, 3];
    let mut recorder = MockSequenceRecorder::new(<[u32; 3] as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<[u32; 3] as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: [u32; 3] = <[u32; 3] as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, [4, 5, 6]);
}

#[test]
fn test_array_recv_rejects_length_mismatch() {
    let mut loader = MockSequenceLoader::new(<[u32; 3] as TypeShape>::SCHEMA, [4u32, 5]);
    let err = <[u32; 3] as TypeShape>::recv(Version::zero(), &mut loader).unwrap_err();
    assert_eq!(err.to_string(), "array length mismatch: expected 3, got 2");
}

#[test]
fn test_vec_deque_send_and_recv_use_sequence_visitors_in_order() {
    let value = VecDeque::from([1u32, 2, 3]);
    let mut recorder = MockSequenceRecorder::new(<VecDeque<u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<VecDeque<u32> as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: VecDeque<u32> =
        <VecDeque<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, VecDeque::from([4u32, 5, 6]));
}

#[test]
fn test_linked_list_send_and_recv_use_sequence_visitors_in_order() {
    let value: LinkedList<u32> = [1u32, 2, 3].into_iter().collect();
    let mut recorder = MockSequenceRecorder::new(<LinkedList<u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<LinkedList<u32> as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: LinkedList<u32> =
        <LinkedList<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded.into_iter().collect::<Vec<_>>(), vec![4, 5, 6]);
}

#[test]
fn test_hash_set_send_and_recv_use_sequence_visitors() {
    let value = HashSet::from([1u32, 2, 3]);
    let mut recorder = MockSequenceRecorder::new(<HashSet<u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();
    recorder.values.sort_unstable();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<HashSet<u32> as TypeShape>::SCHEMA, [4u32, 5, 6]);
    let decoded: HashSet<u32> =
        <HashSet<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, HashSet::from([4u32, 5, 6]));
}

#[test]
fn test_btree_set_send_and_recv_use_sequence_visitors() {
    let value = BTreeSet::from([3u32, 1, 2]);
    let mut recorder = MockSequenceRecorder::new(<BTreeSet<u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.values, vec![1, 2, 3]);

    let mut loader = MockSequenceLoader::new(<BTreeSet<u32> as TypeShape>::SCHEMA, [6u32, 4, 5]);
    let decoded: BTreeSet<u32> =
        <BTreeSet<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, BTreeSet::from([4u32, 5, 6]));
}

#[test]
fn test_hash_set_recv_rejects_duplicate_elements() {
    let mut loader = MockSequenceLoader::new(<HashSet<u32> as TypeShape>::SCHEMA, [7u32, 7]);
    let err = <HashSet<u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap_err();
    assert_eq!(err.to_string(), "duplicate set element received");
}

#[test]
fn test_hash_map_send_and_recv_use_map_visitors() {
    let value = HashMap::from([(2u32, 20u32), (1, 10)]);
    let mut recorder = MockMapRecorder::new(<HashMap<u32, u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();
    recorder.entries.sort_unstable();

    assert_eq!(recorder.entries, vec![(1, 10), (2, 20)]);

    let mut loader = MockMapLoader::new(
        <HashMap<u32, u32> as TypeShape>::SCHEMA,
        [(4u32, 40u32), (3, 30)],
    );
    let decoded: HashMap<u32, u32> =
        <HashMap<u32, u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, HashMap::from([(3u32, 30u32), (4, 40)]));
}

#[test]
fn test_btree_map_send_and_recv_use_map_visitors() {
    let value = BTreeMap::from([(2u32, 20u32), (1, 10)]);
    let mut recorder = MockMapRecorder::new(<BTreeMap<u32, u32> as TypeShape>::SCHEMA);

    value.send(Version::zero(), &mut recorder).unwrap();

    assert_eq!(recorder.entries, vec![(1, 10), (2, 20)]);

    let mut loader = MockMapLoader::new(
        <BTreeMap<u32, u32> as TypeShape>::SCHEMA,
        [(4u32, 40u32), (3, 30)],
    );
    let decoded: BTreeMap<u32, u32> =
        <BTreeMap<u32, u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap();
    assert_eq!(decoded, BTreeMap::from([(3u32, 30u32), (4, 40)]));
}

#[test]
fn test_hash_map_recv_rejects_duplicate_keys() {
    let mut loader = MockMapLoader::new(
        <HashMap<u32, u32> as TypeShape>::SCHEMA,
        [(9u32, 90u32), (9, 99)],
    );
    let err = <HashMap<u32, u32> as TypeShape>::recv(Version::zero(), &mut loader).unwrap_err();
    assert_eq!(err.to_string(), "duplicate map key received");
}

#[cfg(unix)]
struct MockOwnedFdSendRecorder {
    raw_fd: Option<i32>,
}

#[cfg(unix)]
impl PrimitiveSendVisitor for MockOwnedFdSendRecorder {
    fn visit_bool(&mut self, _v: &bool) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i8(&mut self, _v: &i8) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i16(&mut self, _v: &i16) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i32(&mut self, _v: &i32) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i64(&mut self, _v: &i64) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i128(&mut self, _v: &i128) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u8(&mut self, _v: &u8) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u16(&mut self, _v: &u16) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u32(&mut self, _v: &u32) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u64(&mut self, _v: &u64) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u128(&mut self, _v: &u128) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_f32(&mut self, _v: &f32) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_f64(&mut self, _v: &f64) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_char(&mut self, _v: &char) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_string(&mut self, _v: &String) -> Result<()> {
        Err(anyhow!("unexpected primitive"))
    }

    #[cfg(unix)]
    fn visit_file(&mut self, v: &OwnedFd) -> Result<()> {
        self.raw_fd = Some(v.as_raw_fd());
        Ok(())
    }
}

#[cfg(unix)]
impl SendAccept for MockOwnedFdSendRecorder {
    fn accept_primitive(&mut self, send: &impl kaloron::PrimitiveSend) -> Result<()> {
        send.visit(self)
    }
    fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _send: &impl SequenceSend) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, _send: &impl kaloron::TupleSend) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
    }
    fn accept_map(&mut self, _send: &impl MapSend) -> Result<()> {
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

#[cfg(unix)]
struct MockOwnedFdRecvVisitor {
    value: Option<OwnedFd>,
}

#[cfg(unix)]
impl PrimitiveRecvVisitor for MockOwnedFdRecvVisitor {
    fn visit_bool(&mut self) -> Result<bool> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i8(&mut self) -> Result<i8> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i16(&mut self) -> Result<i16> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i32(&mut self) -> Result<i32> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i64(&mut self) -> Result<i64> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_i128(&mut self) -> Result<i128> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u8(&mut self) -> Result<u8> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u16(&mut self) -> Result<u16> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u32(&mut self) -> Result<u32> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u64(&mut self) -> Result<u64> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_u128(&mut self) -> Result<u128> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_f32(&mut self) -> Result<f32> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_f64(&mut self) -> Result<f64> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_char(&mut self) -> Result<char> {
        Err(anyhow!("unexpected primitive"))
    }
    fn visit_string(&mut self) -> Result<String> {
        Err(anyhow!("unexpected primitive"))
    }

    #[cfg(unix)]
    fn visit_file(&mut self) -> Result<OwnedFd> {
        let value = self.value.take().expect("fd already consumed");
        Ok(value)
    }
}

#[cfg(unix)]
impl RecvAccept for MockOwnedFdRecvVisitor {
    fn accept_primitive(&mut self, recv: &mut impl kaloron::PrimitiveRecv) -> Result<()> {
        recv.visit(self)
    }
    fn accept_option(&mut self, _recv: &mut impl kaloron::OptionRecv) -> Result<()> {
        Err(anyhow!("unexpected option"))
    }
    fn accept_sequence(&mut self, _recv: &mut impl SequenceRecv) -> Result<()> {
        Err(anyhow!("unexpected sequence"))
    }
    fn accept_tuple(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple"))
    }
    fn accept_map(&mut self, _recv: &mut impl MapRecv) -> Result<()> {
        Err(anyhow!("unexpected map"))
    }
    fn accept_newtype_struct(&mut self, _recv: &mut impl kaloron::NewTypeRecv) -> Result<()> {
        Err(anyhow!("unexpected newtype struct"))
    }
    fn accept_tuple_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected tuple struct"))
    }
    fn accept_named_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> Result<()> {
        Err(anyhow!("unexpected struct"))
    }
    fn accept_enum(&mut self, _recv: &mut impl kaloron::EnumRecv) -> Result<()> {
        Err(anyhow!("unexpected enum"))
    }
}

#[cfg(unix)]
#[test]
fn test_file_primitive_schema_send_and_recv() {
    assert!(matches!(
        <OwnedFd as TypeShape>::SCHEMA,
        Schema::Primitive(Primitive::File)
    ));

    let send_file = File::open("/dev/null").unwrap();
    let send_expected_fd = send_file.as_raw_fd();
    let send_value: OwnedFd = send_file.into();
    let mut sender = MockOwnedFdSendRecorder { raw_fd: None };
    send_value.send(Version::zero(), &mut sender).unwrap();
    assert_eq!(sender.raw_fd, Some(send_expected_fd));

    let recv_file = File::open("/dev/null").unwrap();
    let recv_expected_fd = recv_file.as_raw_fd();
    let mut recv = MockOwnedFdRecvVisitor {
        value: Some(recv_file.into()),
    };
    let decoded: OwnedFd = <OwnedFd as TypeShape>::recv(Version::zero(), &mut recv).unwrap();
    assert_eq!(decoded.as_raw_fd(), recv_expected_fd);
}
