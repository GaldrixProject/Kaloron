// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{
    MarkerCompositeTypeShapeEnum, Primitive, Schema, SendAccept, TypeShape, VariantKind, Version,
};
use std::mem::ManuallyDrop;

#[allow(dead_code)]
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub enum ExampleEnum {
    #[kaloron(id = 0)]
    Unit,
    #[kaloron(id = 1)]
    Newtype(u32),
    #[kaloron(id = 2)]
    Tuple(u32, String),
    #[kaloron(id = 3)]
    Struct {
        #[kaloron(id = 0)]
        id: u32,
        #[kaloron(id = 1)]
        inner: u32,
    },
}

fn assist_assert_enum_marker<T: MarkerCompositeTypeShapeEnum>() {}

#[test]
fn test_example_enum_has_expected_enum_shape() {
    assist_assert_enum_marker::<ExampleEnum>();
    let schema = match ExampleEnum::SCHEMA {
        Schema::Enum(schema) => schema,
        other => panic!("expected enum schema, got {other:?}"),
    };

    assert_eq!(schema.name, "ExampleEnum");
    assert_eq!(schema.variants.len(), 4);

    assert_eq!(schema.variants[0].id, 0);
    match &schema.variants[0].kind {
        VariantKind::Unit(u) => assert_eq!(u.name, "Unit"),
        _ => panic!("expected unit variant"),
    }

    assert_eq!(schema.variants[1].id, 1);
    match &schema.variants[1].kind {
        VariantKind::NewType(n) => {
            assert_eq!(n.name, "Newtype");
            assert_eq!(*n.inner, Schema::Primitive(Primitive::U32));
        }
        _ => panic!("expected newtype variant"),
    }

    assert_eq!(schema.variants[2].id, 2);
    match &schema.variants[2].kind {
        VariantKind::Tuple(t) => {
            assert_eq!(t.name, "Tuple");
            assert_eq!(t.elems.len(), 2);
            assert_eq!(*t.elems[0], Schema::Primitive(Primitive::U32));
            assert_eq!(*t.elems[1], Schema::Primitive(Primitive::String));
        }
        _ => panic!("expected tuple variant"),
    }

    assert_eq!(schema.variants[3].id, 3);
    match &schema.variants[3].kind {
        VariantKind::Named(s) => {
            assert_eq!(s.name, "Struct");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "id");
            assert_eq!(*s.fields[0].ty, Schema::Primitive(Primitive::U32));
            assert_eq!(s.fields[1].name, "inner");
            assert_eq!(*s.fields[1].ty, Schema::Primitive(Primitive::U32));
        }
        _ => panic!("expected struct variant"),
    }
}

struct MockSingleValueVisitor<T> {
    value: ::core::option::Option<T>,
}

impl<T> MockSingleValueVisitor<T> {
    fn new(value: T) -> Self {
        Self {
            value: ::core::option::Option::Some(value),
        }
    }
}

impl<T: 'static> kaloron::RecvVisitor for MockSingleValueVisitor<T> {
    fn visit<U>(&mut self) -> anyhow::Result<U> {
        let value = self.value.take().expect("missing visitor value");
        let value = ManuallyDrop::new(value);
        let ptr = (&*value as *const T).cast::<U>();
        Ok(unsafe { ptr.read() })
    }
}

struct MockExampleEnumRecv {
    value: ExampleEnum,
}

struct MockUnitEnumAccept;

impl kaloron::EnumRecvAccept for MockUnitEnumAccept {
    fn accept_newtype(&mut self, _visitor: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_named(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }
}

struct MockNewtypeEnumAccept {
    value: u32,
}

impl kaloron::EnumRecvAccept for MockNewtypeEnumAccept {
    fn accept_newtype(&mut self, visitor: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        let mut inner = MockSingleValueVisitor::new(self.value);
        visitor.visit(&mut inner)
    }

    fn accept_tuple(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_named(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }
}

struct MockTupleEnumAccept {
    number: u32,
    text: String,
}

impl kaloron::EnumRecvAccept for MockTupleEnumAccept {
    fn accept_newtype(&mut self, _visitor: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple(&mut self, visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        let mut first = MockSingleValueVisitor::new(self.number);
        visitor.visit(0, &mut first)?;
        let mut second = MockSingleValueVisitor::new(self.text.clone());
        visitor.visit(1, &mut second)
    }

    fn accept_named(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }
}

struct MockStructEnumAccept {
    id: u32,
    inner: u32,
}

impl kaloron::EnumRecvAccept for MockStructEnumAccept {
    fn accept_newtype(&mut self, _visitor: &mut impl kaloron::NewTypeRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple(&mut self, _visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_named(&mut self, visitor: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        let mut first = MockSingleValueVisitor::new(self.id);
        visitor.visit(0, &mut first)?;
        let mut second = MockSingleValueVisitor::new(self.inner);
        visitor.visit(1, &mut second)
    }
}

impl kaloron::RecvAccept for MockExampleEnumRecv {
    fn accept_primitive(&mut self, _recv: &mut impl kaloron::PrimitiveRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_option(&mut self, _recv: &mut impl kaloron::OptionRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_sequence(&mut self, _recv: &mut impl kaloron::SequenceRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple(&mut self, _recv: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_map(&mut self, _recv: &mut impl kaloron::MapRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_newtype_struct(
        &mut self,
        _recv: &mut impl kaloron::NewTypeRecv,
    ) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_named_struct(&mut self, _recv: &mut impl kaloron::TupleRecv) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_enum(&mut self, recv: &mut impl kaloron::EnumRecv) -> anyhow::Result<()> {
        match &self.value {
            ExampleEnum::Unit => {
                let mut accept = MockUnitEnumAccept;
                recv.visit(0, &mut accept)
            }
            ExampleEnum::Newtype(value) => {
                let mut accept = MockNewtypeEnumAccept { value: *value };
                recv.visit(1, &mut accept)
            }
            ExampleEnum::Tuple(number, text) => {
                let mut accept = MockTupleEnumAccept {
                    number: *number,
                    text: text.clone(),
                };
                recv.visit(2, &mut accept)
            }
            ExampleEnum::Struct { id, inner } => {
                let mut accept = MockStructEnumAccept {
                    id: *id,
                    inner: *inner,
                };
                recv.visit(3, &mut accept)
            }
        }
    }
}
#[test]
fn test_example_enum_recv_handles_all_variant_kinds() {
    let cases = [
        ExampleEnum::Unit,
        ExampleEnum::Newtype(7),
        ExampleEnum::Tuple(11, "tuple".to_string()),
        ExampleEnum::Struct { id: 42, inner: 99 },
    ];

    for case in cases {
        let mut recv = MockExampleEnumRecv { value: case };
        let value = <ExampleEnum as TypeShape>::recv(Version::zero(), &mut recv).unwrap();
        assert_eq!(value, recv.value);
    }
}

#[derive(Debug, PartialEq)]
enum MockSentValue {
    U32(u32),
    String(String),
}

#[derive(Debug, PartialEq)]
enum MockSentVariant {
    Unit {
        index: isize,
    },
    Newtype {
        index: isize,
        values: Vec<MockSentValue>,
    },
    Tuple {
        index: isize,
        values: Vec<MockSentValue>,
    },
    Struct {
        index: isize,
        values: Vec<MockSentValue>,
    },
}

#[derive(Default)]
struct MockPayloadRecorder {
    values: Vec<MockSentValue>,
}

impl kaloron::SendVisitor for MockPayloadRecorder {
    fn visit<T>(&mut self, value: &T) -> anyhow::Result<()> {
        if std::any::type_name::<T>() == std::any::type_name::<u32>() {
            let value = unsafe { &*(value as *const T).cast::<u32>() };
            self.values.push(MockSentValue::U32(*value));
            return Ok(());
        }

        if std::any::type_name::<T>() == std::any::type_name::<String>() {
            let value = unsafe { &*(value as *const T).cast::<String>() };
            self.values.push(MockSentValue::String(value.clone()));
            return Ok(());
        }

        anyhow::bail!(
            "unsupported send type in ExampleEnum test: {}",
            std::any::type_name::<T>()
        )
    }
}

fn assist_collect_newtype_payload(visitor: &impl kaloron::NewTypeSend) -> anyhow::Result<Vec<MockSentValue>> {
    let mut recorder = MockPayloadRecorder::default();
    visitor.visit(&mut recorder)?;
    Ok(recorder.values)
}

fn assist_collect_tuple_payload(
    visitor: &impl kaloron::TupleSend,
    len: isize,
) -> anyhow::Result<Vec<MockSentValue>> {
    let mut recorder = MockPayloadRecorder::default();
    for index in 0..len {
        visitor.visit(index, &mut recorder)?;
    }
    Ok(recorder.values)
}

#[derive(Default)]
struct MockExampleEnumSendRecorder {
    variant: Option<MockSentVariant>,
}

impl kaloron::EnumSendAccept for MockExampleEnumSendRecorder {
    fn accept_unit(&mut self, index: isize) -> anyhow::Result<()> {
        self.variant = Some(MockSentVariant::Unit { index });
        Ok(())
    }

    fn accept_newtype(
        &mut self,
        index: isize,
        visitor: &impl kaloron::NewTypeSend,
    ) -> anyhow::Result<()> {
        self.variant = Some(MockSentVariant::Newtype {
            index,
            values: assist_collect_newtype_payload(visitor)?,
        });
        Ok(())
    }

    fn accept_tuple(
        &mut self,
        index: isize,
        visitor: &impl kaloron::TupleSend,
    ) -> anyhow::Result<()> {
        self.variant = Some(MockSentVariant::Tuple {
            index,
            values: assist_collect_tuple_payload(visitor, 2)?,
        });
        Ok(())
    }

    fn accept_named(
        &mut self,
        index: isize,
        visitor: &impl kaloron::TupleSend,
    ) -> anyhow::Result<()> {
        self.variant = Some(MockSentVariant::Struct {
            index,
            values: assist_collect_tuple_payload(visitor, 2)?,
        });
        Ok(())
    }
}

impl SendAccept for MockExampleEnumSendRecorder {
    fn accept_primitive(&mut self, _send: &impl kaloron::PrimitiveSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_option(&mut self, _send: &impl kaloron::OptionSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_sequence(&mut self, _send: &impl kaloron::SequenceSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple(&mut self, _send: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_map(&mut self, _send: &impl kaloron::MapSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_newtype_struct(&mut self, _send: &impl kaloron::NewTypeSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_tuple_struct(&mut self, _send: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_named_struct(&mut self, _send: &impl kaloron::TupleSend) -> anyhow::Result<()> {
        anyhow::bail!("not used in ExampleEnum test")
    }

    fn accept_enum(&mut self, send: &impl kaloron::EnumSend) -> anyhow::Result<()> {
        send.visit(self)
    }
}
#[test]
fn test_example_enum_send_handles_all_variant_kinds() {
    let cases = [
        (ExampleEnum::Unit, MockSentVariant::Unit { index: 0 }),
        (
            ExampleEnum::Newtype(7),
            MockSentVariant::Newtype {
                index: 1,
                values: vec![MockSentValue::U32(7)],
            },
        ),
        (
            ExampleEnum::Tuple(11, "tuple".to_string()),
            MockSentVariant::Tuple {
                index: 2,
                values: vec![MockSentValue::U32(11), MockSentValue::String("tuple".to_string())],
            },
        ),
        (
            ExampleEnum::Struct { id: 42, inner: 99 },
            MockSentVariant::Struct {
                index: 3,
                values: vec![MockSentValue::U32(42), MockSentValue::U32(99)],
            },
        ),
    ];

    for (value, expected) in cases {
        let mut recorder = MockExampleEnumSendRecorder::default();
        value.send(Version::zero(), &mut recorder).unwrap();
        assert_eq!(recorder.variant, Some(expected));
    }
}
