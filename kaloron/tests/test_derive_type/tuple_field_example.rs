// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{Primitive, Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct TupleFieldExample {
    #[kaloron(id = 0)]
    pub pair: (u32, String),
    #[kaloron(id = 1)]
    pub label: String,
}

#[test]
fn test_tuple_field_example_has_struct_shape_with_tuple_field() {
    match TupleFieldExample::SCHEMA {
        Schema::Named(schema) => {
            assert_eq!(schema.name, "TupleFieldExample");
            assert_eq!(schema.fields.len(), 2);
            assert_eq!(schema.fields[0].name, "pair");
            match schema.fields[0].ty {
                Schema::Tuple(elems) => {
                    assert_eq!(elems.len(), 2);
                    assert_eq!(*elems[0], Schema::Primitive(Primitive::U32));
                    assert_eq!(*elems[1], Schema::Primitive(Primitive::String));
                }
                other => panic!("expected tuple field schema, got {other:?}"),
            }
            assert_eq!(schema.fields[1].name, "label");
            assert_eq!(*schema.fields[1].ty, Schema::Primitive(Primitive::String));
        }
        other => panic!("expected struct schema, got {other:?}"),
    }
}
