// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{Primitive, Schema, TypeShape};

use super::derived_example::DerivedExample;

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct NestedExample {
    #[kaloron(id = 0)]
    pub inner: DerivedExample,
    #[kaloron(id = 1)]
    pub note: String,
}

#[test]
fn test_nested_example_has_struct_shape_with_nested_type_shape() {
    match NestedExample::SCHEMA {
        Schema::Named(schema) => {
            assert_eq!(schema.name, "NestedExample");
            assert_eq!(schema.fields.len(), 2);
            assert_eq!(schema.fields[0].name, "inner");
            assert_eq!(*schema.fields[0].ty, *DerivedExample::SCHEMA);
            assert_eq!(schema.fields[1].name, "note");
            assert_eq!(*schema.fields[1].ty, Schema::Primitive(Primitive::String));
        }
        other => panic!("expected struct schema, got {other:?}"),
    }
}
