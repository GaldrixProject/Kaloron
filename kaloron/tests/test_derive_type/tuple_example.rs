// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{MarkerCompositeTypeShapeTupleStruct, Primitive, Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct TupleExample(pub u32, pub String);

fn assist_assert_tuple_struct_marker<T: MarkerCompositeTypeShapeTupleStruct>() {}

#[test]
fn test_tuple_example_has_tuple_struct_shape() {
    assist_assert_tuple_struct_marker::<TupleExample>();
    match TupleExample::SCHEMA {
        Schema::TupleStruct(tuple) => {
            assert_eq!(tuple.name, "TupleExample");
            assert_eq!(tuple.elems.len(), 2);
            assert_eq!(*tuple.elems[0], Schema::Primitive(Primitive::U32));
            assert_eq!(*tuple.elems[1], Schema::Primitive(Primitive::String));
        }
        other => panic!("expected tuple struct schema, got {other:?}"),
    }
}
