// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{MarkerCompositeTypeShapeNamedStruct, Primitive, Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct DerivedExample {
    #[kaloron(id = 0)]
    pub id: u32,
    #[kaloron(id = 1)]
    pub name: String,
}

fn assist_assert_named_struct_marker<T: MarkerCompositeTypeShapeNamedStruct>() {}

#[test]
fn test_derived_example_has_struct_shape() {
    assist_assert_named_struct_marker::<DerivedExample>();
    let struct_schema = match DerivedExample::SCHEMA {
        Schema::Named(schema) => schema,
        other => panic!("expected struct schema, got {other:?}"),
    };

    assert_eq!(struct_schema.name, "DerivedExample");
    assert_eq!(struct_schema.fields.len(), 2);
    assert_eq!(struct_schema.fields[0].id, 0);
    assert_eq!(struct_schema.fields[0].name, "id");
    assert_eq!(
        *struct_schema.fields[0].ty,
        Schema::Primitive(Primitive::U32)
    );
    assert_eq!(struct_schema.fields[1].id, 1);
    assert_eq!(struct_schema.fields[1].name, "name");
    assert_eq!(
        *struct_schema.fields[1].ty,
        Schema::Primitive(Primitive::String)
    );
    assert_eq!(
        struct_schema.field_by_name("id").map(|f| f.name),
        Some("id")
    );
    assert_eq!(
        struct_schema.field_by_name("name").map(|f| f.name),
        Some("name")
    );
}
