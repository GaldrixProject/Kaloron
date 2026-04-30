// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::{ActivationState, Gated, Schema, TypeShape, VariantKind, Version};

// ---------------------------------------------------------------------------
// Named struct with fields at different versions
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct VersionedStruct {
    #[kaloron(id = 0, introduced = "1.0.0")]
    pub base: u32,

    #[kaloron(id = 1, introduced = "1.2.0")]
    pub added_later: Gated<String>,

    #[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
    pub temporary: Gated<bool>,
}

#[test]
fn test_named_struct_has_field_activation_table() {
    let schema = match VersionedStruct::SCHEMA {
        Schema::Named(s) => s,
        other => panic!("expected named schema, got {other:?}"),
    };

    assert_eq!(schema.fields.len(), 3);
    assert!(!schema.field_activations.is_empty());

    // The change points are: 1.0.0, 1.2.0, 1.5.0, 3.0.0
    assert_eq!(schema.field_activations.len(), 4);

    // 1.0.0: only base active
    assert_eq!(
        schema.field_activations[0].version,
        Version::new(1, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[0].active,
        &[
            ActivationState::Active,
            ActivationState::Inactive,
            ActivationState::Inactive
        ]
    );

    // 1.2.0: base + added_later active
    assert_eq!(
        schema.field_activations[1].version,
        Version::new(1, 2, 0, 0)
    );
    assert_eq!(
        schema.field_activations[1].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Inactive
        ]
    );

    // 1.5.0: all three active
    assert_eq!(
        schema.field_activations[2].version,
        Version::new(1, 5, 0, 0)
    );
    assert_eq!(
        schema.field_activations[2].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Active
        ]
    );

    // 3.0.0: temporary removed
    assert_eq!(
        schema.field_activations[3].version,
        Version::new(3, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[3].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Inactive
        ]
    );
}

#[test]
fn test_named_struct_active_fields_at_binary_search() {
    let schema = match VersionedStruct::SCHEMA {
        Schema::Named(s) => s,
        other => panic!("expected named schema, got {other:?}"),
    };

    // Before any entry
    assert_eq!(schema.active_fields_at(Version::new(0, 9, 0, 0)), None);

    // Exact match on first entry
    assert_eq!(
        schema.active_fields_at(Version::new(1, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Inactive,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // Between first and second entry
    assert_eq!(
        schema.active_fields_at(Version::new(1, 1, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Inactive,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // Exact match on second entry
    assert_eq!(
        schema.active_fields_at(Version::new(1, 2, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // Between second and third entry
    assert_eq!(
        schema.active_fields_at(Version::new(1, 3, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // All active
    assert_eq!(
        schema.active_fields_at(Version::new(2, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Active
            ]
            .as_slice()
        )
    );

    // After removal
    assert_eq!(
        schema.active_fields_at(Version::new(3, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // Far future
    assert_eq!(
        schema.active_fields_at(Version::new(10, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );
}

// ---------------------------------------------------------------------------
// Named struct where all fields share same version (simple case)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct SimpleVersionedStruct {
    #[kaloron(id = 0)]
    pub x: u32,
    #[kaloron(id = 1)]
    pub y: u32,
}

#[test]
fn test_simple_struct_has_single_activation_entry() {
    let schema = match SimpleVersionedStruct::SCHEMA {
        Schema::Named(s) => s,
        other => panic!("expected named schema, got {other:?}"),
    };

    // All fields inherit 1.0.0, so only one change point
    assert_eq!(schema.field_activations.len(), 1);
    assert_eq!(
        schema.field_activations[0].version,
        Version::new(1, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[0].active,
        &[ActivationState::Active, ActivationState::Active]
    );
}

// ---------------------------------------------------------------------------
// Struct with a deprecated field
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct DeprecatedFieldStruct {
    #[kaloron(id = 0, introduced = "1.0.0", deprecated = "2.0.0..2.1.0")]
    pub legacy: u32,

    #[kaloron(id = 1, introduced = "1.0.0")]
    pub current: u32,
}

#[test]
fn test_deprecated_field_activation_table_marks_deprecated_states() {
    let schema = match DeprecatedFieldStruct::SCHEMA {
        Schema::Named(s) => s,
        other => panic!("expected named schema, got {other:?}"),
    };

    assert_eq!(schema.field_activations.len(), 3);

    // 1.0.0: both active
    assert_eq!(
        schema.field_activations[0].version,
        Version::new(1, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[0].active,
        &[ActivationState::Active, ActivationState::Active,]
    );

    // 2.0.0: legacy becomes deprecated
    assert_eq!(
        schema.field_activations[1].version,
        Version::new(2, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[1].active,
        &[ActivationState::Deprecated, ActivationState::Active,]
    );

    // 2.1.0.1: deprecation window ended, legacy active again
    assert_eq!(
        schema.field_activations[2].version,
        Version::new(2, 1, 0, 1)
    );
    assert_eq!(
        schema.field_activations[2].active,
        &[ActivationState::Active, ActivationState::Active,]
    );

    assert_eq!(
        schema.active_fields_at(Version::new(2, 0, 0, 0)),
        Some([ActivationState::Deprecated, ActivationState::Active].as_slice())
    );
    assert_eq!(
        schema.active_fields_at(Version::new(2, 1, 0, 0)),
        Some([ActivationState::Deprecated, ActivationState::Active].as_slice())
    );
    assert_eq!(
        schema.active_fields_at(Version::new(2, 1, 0, 1)),
        Some([ActivationState::Active, ActivationState::Active].as_slice())
    );
}

// ---------------------------------------------------------------------------
// Enum with variants at different versions
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub enum VersionedEnum {
    #[kaloron(id = 0, introduced = "1.0.0")]
    Alpha,

    #[kaloron(id = 1, introduced = "1.1.0")]
    Beta(u32),

    #[kaloron(id = 2, introduced = "2.0.0", removed = "4.0.0")]
    Gamma {
        #[kaloron(id = 0)]
        id: u32,
        #[kaloron(id = 1)]
        name: String,
    },
}

#[test]
fn test_enum_has_variant_activation_table() {
    let schema = match VersionedEnum::SCHEMA {
        Schema::Enum(s) => s,
        other => panic!("expected enum schema, got {other:?}"),
    };

    assert_eq!(schema.variants.len(), 3);
    assert!(!schema.variant_activations.is_empty());

    // Change points: 1.0.0, 1.1.0, 2.0.0, 4.0.0
    assert_eq!(schema.variant_activations.len(), 4);

    // 1.0.0: only Alpha
    assert_eq!(
        schema.variant_activations[0].version,
        Version::new(1, 0, 0, 0)
    );
    assert_eq!(
        schema.variant_activations[0].active,
        &[
            ActivationState::Active,
            ActivationState::Inactive,
            ActivationState::Inactive
        ]
    );

    // 1.1.0: Alpha + Beta
    assert_eq!(
        schema.variant_activations[1].version,
        Version::new(1, 1, 0, 0)
    );
    assert_eq!(
        schema.variant_activations[1].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Inactive
        ]
    );

    // 2.0.0: all three
    assert_eq!(
        schema.variant_activations[2].version,
        Version::new(2, 0, 0, 0)
    );
    assert_eq!(
        schema.variant_activations[2].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Active
        ]
    );

    // 4.0.0: Gamma removed
    assert_eq!(
        schema.variant_activations[3].version,
        Version::new(4, 0, 0, 0)
    );
    assert_eq!(
        schema.variant_activations[3].active,
        &[
            ActivationState::Active,
            ActivationState::Active,
            ActivationState::Inactive
        ]
    );
}

#[test]
fn test_enum_active_variants_at_binary_search() {
    let schema = match VersionedEnum::SCHEMA {
        Schema::Enum(s) => s,
        other => panic!("expected enum schema, got {other:?}"),
    };

    // Before any variant
    assert_eq!(schema.active_variants_at(Version::new(0, 9, 0, 0)), None);

    // Exact match
    assert_eq!(
        schema.active_variants_at(Version::new(1, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Inactive,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // Between entries
    assert_eq!(
        schema.active_variants_at(Version::new(1, 0, 5, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Inactive,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );

    // All active
    assert_eq!(
        schema.active_variants_at(Version::new(3, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Active
            ]
            .as_slice()
        )
    );

    // After removal
    assert_eq!(
        schema.active_variants_at(Version::new(5, 0, 0, 0)),
        Some(
            [
                ActivationState::Active,
                ActivationState::Active,
                ActivationState::Inactive
            ]
            .as_slice()
        )
    );
}

// ---------------------------------------------------------------------------
// Named variant within enum has its own field activation table
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub enum EnumWithNamedVariant {
    #[kaloron(id = 0)]
    Unit,
    #[kaloron(id = 1)]
    Named {
        #[kaloron(id = 0, introduced = "1.0.0")]
        always: u32,

        #[kaloron(id = 1, introduced = "2.0.0")]
        added: Gated<String>,
    },
}

#[test]
fn test_named_variant_has_field_activation_table() {
    let schema = match EnumWithNamedVariant::SCHEMA {
        Schema::Enum(s) => s,
        other => panic!("expected enum schema, got {other:?}"),
    };

    assert_eq!(schema.variants.len(), 2);

    // Check the Named variant's field activations
    let named_schema = match &schema.variants[1].kind {
        VariantKind::Named(s) => s,
        other => panic!("expected named variant, got {other:?}"),
    };

    assert_eq!(named_schema.fields.len(), 2);
    assert_eq!(named_schema.field_activations.len(), 2);

    // 1.0.0: only always
    assert_eq!(
        named_schema.field_activations[0].version,
        Version::new(1, 0, 0, 0)
    );
    assert_eq!(
        named_schema.field_activations[0].active,
        &[ActivationState::Active, ActivationState::Inactive]
    );

    // 2.0.0: both active
    assert_eq!(
        named_schema.field_activations[1].version,
        Version::new(2, 0, 0, 0)
    );
    assert_eq!(
        named_schema.field_activations[1].active,
        &[ActivationState::Active, ActivationState::Active]
    );
}

#[test]
fn test_named_variant_active_fields_at_binary_search() {
    let schema = match EnumWithNamedVariant::SCHEMA {
        Schema::Enum(s) => s,
        other => panic!("expected enum schema, got {other:?}"),
    };

    let named_schema = match &schema.variants[1].kind {
        VariantKind::Named(s) => s,
        other => panic!("expected named variant, got {other:?}"),
    };

    assert_eq!(
        named_schema.active_fields_at(Version::new(0, 5, 0, 0)),
        None
    );
    assert_eq!(
        named_schema.active_fields_at(Version::new(1, 0, 0, 0)),
        Some([ActivationState::Active, ActivationState::Inactive].as_slice())
    );
    assert_eq!(
        named_schema.active_fields_at(Version::new(1, 5, 0, 0)),
        Some([ActivationState::Active, ActivationState::Inactive].as_slice())
    );
    assert_eq!(
        named_schema.active_fields_at(Version::new(2, 0, 0, 0)),
        Some([ActivationState::Active, ActivationState::Active].as_slice())
    );
    assert_eq!(
        named_schema.active_fields_at(Version::new(5, 0, 0, 0)),
        Some([ActivationState::Active, ActivationState::Active].as_slice())
    );
}

// ---------------------------------------------------------------------------
// Inheritance: fields without version attrs inherit from container
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, TypeShape)]
#[kaloron(introduced = "2.0.0", removed = "5.0.0")]
pub struct InheritedVersionStruct {
    #[kaloron(id = 0)]
    pub inherited_field: u32,

    #[kaloron(id = 1, introduced = "3.0.0")]
    pub later_field: Gated<String>,
}

#[test]
fn test_inherited_version_produces_correct_activation_table() {
    let schema = match InheritedVersionStruct::SCHEMA {
        Schema::Named(s) => s,
        other => panic!("expected named schema, got {other:?}"),
    };

    // Change points: 2.0.0 (inherited intro), 3.0.0 (later intro), 5.0.0 (inherited removal)
    assert_eq!(schema.field_activations.len(), 3);

    // 2.0.0: inherited_field active, later_field not yet introduced
    assert_eq!(
        schema.field_activations[0].version,
        Version::new(2, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[0].active,
        &[ActivationState::Active, ActivationState::Inactive]
    );

    // 3.0.0: both active
    assert_eq!(
        schema.field_activations[1].version,
        Version::new(3, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[1].active,
        &[ActivationState::Active, ActivationState::Active]
    );

    // 5.0.0: both removed (inherited from container)
    assert_eq!(
        schema.field_activations[2].version,
        Version::new(5, 0, 0, 0)
    );
    assert_eq!(
        schema.field_activations[2].active,
        &[ActivationState::Inactive, ActivationState::Inactive]
    );
}
