// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{Primitive, Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct GenericExample<T> {
    #[kaloron(id = 0)]
    pub value: T,
    #[kaloron(id = 1)]
    pub note: String,
}

#[test]
fn test_generic_example_has_struct_shape_for_concrete_type() {
    let struct_schema = match GenericExample::<u32>::SCHEMA {
        Schema::Named(schema) => schema,
        other => panic!("expected struct schema, got {other:?}"),
    };

    assert_eq!(struct_schema.name, "GenericExample");
    assert_eq!(struct_schema.fields.len(), 2);
    assert_eq!(struct_schema.fields[0].name, "value");
    assert_eq!(
        *struct_schema.fields[0].ty,
        Schema::Primitive(Primitive::U32)
    );
    assert_eq!(struct_schema.fields[1].name, "note");
    assert_eq!(
        *struct_schema.fields[1].ty,
        Schema::Primitive(Primitive::String)
    );
}
