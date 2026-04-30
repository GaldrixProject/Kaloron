// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{MarkerCompositeTypeShapeNewTypeStruct, Primitive, Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct NewtypeExample(pub u32);

fn assist_assert_newtype_struct_marker<T: MarkerCompositeTypeShapeNewTypeStruct>() {}

#[test]
fn test_newtype_example_has_newtype_struct_shape() {
    assist_assert_newtype_struct_marker::<NewtypeExample>();
    match NewtypeExample::SCHEMA {
        Schema::NewTypeStruct(newtype) => {
            assert_eq!(newtype.name, "NewtypeExample");
            assert_eq!(*newtype.inner, Schema::Primitive(Primitive::U32));
        }
        other => panic!("expected newtype struct schema, got {other:?}"),
    }
}
