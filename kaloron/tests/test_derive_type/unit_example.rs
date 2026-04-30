// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{Schema, TypeShape};

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "0.0.0")]
pub struct UnitExample;

#[test]
fn test_unit_example_has_unit_struct_shape() {
    match UnitExample::SCHEMA {
        Schema::UnitStruct(unit) => assert_eq!(unit.name, "UnitExample"),
        other => panic!("expected unit struct schema, got {other:?}"),
    }
}
