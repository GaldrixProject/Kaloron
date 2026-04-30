// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Representation of a value's schema following Serde's data model.
//!
//! `Schema` is a borrowed, non-allocating representation designed to describe
//! the shapes of values that can be serialized/deserialized. It intentionally
//! avoids references and shared pointers and uses borrowed `&'a` references for
//! nested types so schemas can be stored statically.

use blake3::Hasher;
mod version;
use self::version::binary_search_activation;
pub use version::*;

// ---------------------------------------------------------------------------
// Type declarations
// ---------------------------------------------------------------------------

/// Primitive schema kinds represent the atomic, non-structured types that
/// appear frequently in Rust code (integers, floats, bool, char, strings).
/// Byte buffers are modeled as sequences instead of primitives so they can
/// share the same `Schema::Seq` shape as other container types.
/// These are intentionally grouped into a small enum so structured schema
/// variants (like `Struct`, `Enum`, `Seq`, ...) can refer to them via a
/// single `Schema::Primitive` variant.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    // integer types
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,

    // floating point
    F32,
    F64,

    // other primitives
    Char,

    /// UTF-8 string
    String,

    /// File descriptor / file handle (Unix-like systems only).
    #[cfg(unix)]
    File,
}

/// Unit struct schema (named type with no fields)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitStructSchema<'a> {
    /// Name for the unit struct (e.g., fully-qualified path)
    pub name: &'a str,
    /// Version lifecycle metadata.
    pub version: VersionRange,
}

/// Newtype struct schema (single unnamed field)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTypeStructSchema<'a> {
    /// Name for the newtype struct
    pub name: &'a str,
    /// Inner type schema
    pub inner: &'a Schema<'a>,
    /// Version lifecycle metadata.
    pub version: VersionRange,
}

/// Tuple struct schema (named, but fields are positional)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleStructSchema<'a> {
    /// Name for the tuple struct
    pub name: &'a str,
    /// Element schemas
    pub elems: &'a [&'a Schema<'a>],
    /// Version lifecycle metadata.
    pub version: VersionRange,
}

/// Field in a struct
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    /// Explicit field ID for wire stability.
    pub id: u32,
    /// Field name as a borrowed `&str`.
    pub name: &'a str,
    /// Field type schema (borrowed reference to another `Schema`).
    pub ty: &'a Schema<'a>,
    /// Version lifecycle metadata for this field.
    pub version: VersionRange,
}

/// Struct schema with ordered fields and deterministic index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedStructSchema<'a> {
    /// Name for the struct (e.g., fully-qualified path)
    pub name: &'a str,

    /// Ordered list of fields in declaration order.
    pub fields: &'a [Field<'a>],

    /// Deterministic ordered index for looking up fields by name.
    /// This is stored as a static slice of (name, index) pairs sorted by
    /// name to allow binary search lookups without heap allocations.
    index: &'a [(&'a str, usize)],

    /// Version lifecycle metadata.
    pub version: VersionRange,

    /// Version activation table for fields.
    /// Each entry maps a starting version to a bool array indicating which
    /// fields are active from that version onward. Ordered by version ascending.
    pub field_activations: &'a [VersionActivationEntry<'a>],
}

/// Enum schema using Serde variant kinds: unit, newtype, tuple, and named struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumSchema<'a> {
    /// Name for the enum type.
    pub name: &'a str,

    /// Slice of variant schemas.
    pub variants: &'a [Variant<'a>],

    /// Version lifecycle metadata.
    pub version: VersionRange,

    /// Version activation table for variants.
    /// Each entry maps a starting version to a bool array indicating which
    /// variants are active from that version onward. Ordered by version ascending.
    pub variant_activations: &'a [VersionActivationEntry<'a>],
}

/// A single variant in an enum schema, carrying an explicit ID for wire stability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant<'a> {
    /// Explicit variant ID for wire stability.
    pub id: u32,
    /// The kind and payload of this variant.
    pub kind: VariantKind<'a>,
}

/// The kind of an enum variant (unit, newtype, tuple, or named struct).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantKind<'a> {
    /// Unit variant (no data)
    Unit(UnitStructSchema<'a>),
    /// Newtype variant (single value payload)
    NewType(NewTypeStructSchema<'a>),
    /// Tuple variant with element schemas
    Tuple(TupleStructSchema<'a>),
    /// Named struct variant with named fields
    Named(NamedStructSchema<'a>),
}

/// Main schema enum describing all supported shapes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schema<'a> {
    /// Primitive scalar types grouped under `Primitive`.
    Primitive(Primitive),

    /// Option<T>
    Option(&'a Schema<'a>),

    /// Sequence: variably sized sequence of elements (Vec, slice)
    Seq(&'a Schema<'a>),

    /// Tuple: fixed-size heterogeneous tuple / array
    Tuple(&'a [&'a Schema<'a>]),

    /// Map: variably sized mapping (keys and values can be heterogeneous types)
    Map(&'a Schema<'a>, &'a Schema<'a>),

    /// Unit: `()`
    Unit,

    /// Unit struct: named value with no data, e.g., `struct Unit;`
    UnitStruct(UnitStructSchema<'a>),

    /// Newtype struct: named wrapper around one value, e.g., `struct M(u8);`
    NewTypeStruct(NewTypeStructSchema<'a>),

    /// Tuple struct: named tuple, e.g., `struct Rgb(u8,u8,u8);`
    TupleStruct(TupleStructSchema<'a>),

    /// Struct with named fields
    Named(NamedStructSchema<'a>),

    /// Enum with Serde-style variants
    Enum(EnumSchema<'a>),
}

// ---------------------------------------------------------------------------
// Constructors and small helpers
// ---------------------------------------------------------------------------

impl<'a> Field<'a> {
    /// Create a new named field schema reference with default version.
    pub const fn new(id: u32, name: &'a str, ty: &'a Schema<'a>) -> Self {
        Field {
            id,
            name,
            ty,
            version: VersionRange::default(),
        }
    }

    /// Create a new named field schema reference with explicit version.
    pub const fn new_with_version(
        id: u32,
        name: &'a str,
        ty: &'a Schema<'a>,
        version: VersionRange,
    ) -> Self {
        Field {
            id,
            name,
            ty,
            version,
        }
    }

    /// Create a new named field schema using a type that implements `TypeShape`.
    ///
    /// This convenience const fn reads the field type's static schema directly
    /// from the `TypeShape::SCHEMA` associated constant, allowing generated
    /// code to avoid repeating `::<T as TypeShape>::SCHEMA` expressions.
    pub const fn new_typed<T: crate::TypeShape>(id: u32, name: &'a str) -> Self {
        Field {
            id,
            name,
            ty: <T as crate::TypeShape>::SCHEMA,
            version: VersionRange::default(),
        }
    }

    /// Create a typed field with explicit version metadata.
    pub const fn new_typed_with_version<T: crate::TypeShape>(
        id: u32,
        name: &'a str,
        version: VersionRange,
    ) -> Self {
        Field {
            id,
            name,
            ty: <T as crate::TypeShape>::SCHEMA,
            version,
        }
    }
}

impl<'a> NamedStructSchema<'a> {
    /// Create a struct schema with explicit version metadata and field activation table.
    ///
    /// The `field_activations` slice must be ordered by ascending starting version.
    pub const fn new(
        name: &'a str,
        fields: &'a [Field<'a>],
        index: &'a [(&'a str, usize)],
        version: VersionRange,
        field_activations: &'a [VersionActivationEntry<'a>],
    ) -> Self {
        Self {
            name,
            fields,
            index,
            version,
            field_activations,
        }
    }

    /// Lookup a field by name using binary search on the precomputed index
    /// slice. Returns Some(&Field) if found, otherwise None.
    pub fn field_by_name(&self, name: &str) -> Option<&'a Field<'a>> {
        match self.index.binary_search_by(|pair| pair.0.cmp(name)) {
            Ok(i) => {
                let (_, field_idx) = self.index[i];
                Some(&self.fields[field_idx])
            }
            Err(_) => None,
        }
    }

    /// Look up which fields are active at the given version using binary search
    /// on the field activation table.
    /// Returns the active flags array for the version range containing the given
    /// version, or None if the version is before all entries.
    pub fn active_fields_at(&self, version: Version) -> Option<&'a [ActivationState]> {
        binary_search_activation(self.field_activations, version)
    }
}

impl<'a> EnumSchema<'a> {
    /// Create a named enum schema with explicit version metadata and variant activation table.
    ///
    /// The `variant_activations` slice must be ordered by ascending starting version.
    pub const fn new(
        name: &'a str,
        variants: &'a [Variant<'a>],
        version: VersionRange,
        variant_activations: &'a [VersionActivationEntry<'a>],
    ) -> Self {
        EnumSchema {
            name,
            variants,
            version,
            variant_activations,
        }
    }

    /// Look up which variants are active at the given version using binary search
    /// on the variant activation table.
    /// Returns the active flags array for the version range containing the given
    /// version, or None if the version is before all entries.
    pub fn active_variants_at(&self, version: Version) -> Option<&'a [ActivationState]> {
        binary_search_activation(self.variant_activations, version)
    }

    /// Access a variant by its index (id order).
    pub fn variant(&self, index: usize) -> &Variant<'a> {
        &self.variants[index]
    }
}

impl<'a> Variant<'a> {
    /// Create a unit variant schema with explicit version.
    pub const fn unit(id: u32, name: &'a str, version: VersionRange) -> Self {
        Variant {
            id,
            kind: VariantKind::Unit(UnitStructSchema { name, version }),
        }
    }

    /// Create a newtype variant schema with explicit version.
    pub const fn newtype(
        id: u32,
        name: &'a str,
        inner: &'a Schema<'a>,
        version: VersionRange,
    ) -> Self {
        Variant {
            id,
            kind: VariantKind::NewType(NewTypeStructSchema {
                name,
                inner,
                version,
            }),
        }
    }

    /// Create a tuple variant schema with explicit version.
    pub const fn tuple(
        id: u32,
        name: &'a str,
        elems: &'a [&'a Schema<'a>],
        version: VersionRange,
    ) -> Self {
        Variant {
            id,
            kind: VariantKind::Tuple(TupleStructSchema {
                name,
                elems,
                version,
            }),
        }
    }

    /// Create a struct-like variant schema with explicit version and field activation table.
    pub const fn named(
        id: u32,
        name: &'a str,
        fields: &'a [Field<'a>],
        index: &'a [(&'a str, usize)],
        version: VersionRange,
        field_activations: &'a [VersionActivationEntry<'a>],
    ) -> Self {
        Variant {
            id,
            kind: VariantKind::Named(NamedStructSchema {
                name,
                fields,
                index,
                version,
                field_activations,
            }),
        }
    }
}

impl Primitive {
    /// Contribute this primitive kind to a schema hash.
    pub fn hash_build(&self, build: &mut Hasher) {
        let tag = match self {
            Primitive::Bool => 0x20u8,
            Primitive::I8 => 0x21u8,
            Primitive::I16 => 0x22u8,
            Primitive::I32 => 0x23u8,
            Primitive::I64 => 0x24u8,
            Primitive::I128 => 0x25u8,
            Primitive::U8 => 0x26u8,
            Primitive::U16 => 0x27u8,
            Primitive::U32 => 0x28u8,
            Primitive::U64 => 0x29u8,
            Primitive::U128 => 0x2Au8,
            Primitive::F32 => 0x2Bu8,
            Primitive::F64 => 0x2Cu8,
            Primitive::Char => 0x2Du8,
            Primitive::String => 0x2Eu8,
            #[cfg(unix)]
            Primitive::File => 0x2Fu8,
        };
        build.update(&[tag]);
    }
}

impl<'a> UnitStructSchema<'a> {
    /// Contribute this unit struct to a schema hash.
    pub fn hash_build(&self, build: &mut Hasher) {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        self.version.hash_build(build);
    }
}

impl<'a> NewTypeStructSchema<'a> {
    /// Contribute this newtype struct to a schema hash (version-aware).
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        self.inner.hash_build(version, build);
        self.version.hash_build(build);
    }
}

impl<'a> TupleStructSchema<'a> {
    /// Contribute this tuple struct to a schema hash (version-aware).
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        let cnt = self.elems.len() as u64;
        build.update(&cnt.to_be_bytes());
        for elem in self.elems {
            elem.hash_build(version, build);
        }
        self.version.hash_build(build);
    }
}

impl<'a> Field<'a> {
    /// Contribute this field to a schema hash (version-aware).
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        build.update(&self.id.to_be_bytes());
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        self.ty.hash_build(version, build);
        self.version.hash_build(build);
    }
}

impl<'a> NamedStructSchema<'a> {
    /// Contribute this named struct to a schema hash (version-aware).
    ///
    /// Inactive fields (not yet introduced or removed) are excluded entirely.
    /// Active and deprecated fields are both included, but each is prefixed
    /// with a state byte: `0x00` for `Active`, `0x01` for `Deprecated`.
    /// This ensures the hash reflects deprecation transitions across versions.
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        if let Some(states) = self.active_fields_at(version) {
            let included_count = states.iter().filter(|s| !s.is_inactive()).count() as u64;
            build.update(&included_count.to_be_bytes());
            for (i, state) in states.iter().enumerate() {
                if !state.is_inactive() {
                    build.update(&[if state.is_deprecated() {
                        0x01u8
                    } else {
                        0x00u8
                    }]);
                    self.fields[i].hash_build(version, build);
                }
            }
        } else {
            // No activation data for this version: include all fields as active.
            let cnt = self.fields.len() as u64;
            build.update(&cnt.to_be_bytes());
            for field in self.fields {
                build.update(&[0x00u8]);
                field.hash_build(version, build);
            }
        }
        self.version.hash_build(build);
    }
}

impl<'a> Variant<'a> {
    /// Contribute this variant to a schema hash (version-aware).
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        build.update(&self.id.to_be_bytes());
        self.kind.hash_build(version, build);
    }
}

impl<'a> VariantKind<'a> {
    /// Contribute this variant kind to a schema hash (version-aware).
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        match self {
            VariantKind::Unit(u) => {
                build.update(&[0x30]);
                u.hash_build(build);
            }
            VariantKind::NewType(n) => {
                build.update(&[0x31]);
                n.hash_build(version, build);
            }
            VariantKind::Tuple(t) => {
                build.update(&[0x32]);
                t.hash_build(version, build);
            }
            VariantKind::Named(s) => {
                build.update(&[0x33]);
                s.hash_build(version, build);
            }
        }
    }

    /// Returns the inner `UnitStructSchema` if this is a `Unit` variant.
    pub fn as_unit(&self) -> Option<&UnitStructSchema<'a>> {
        match self {
            VariantKind::Unit(u) => Some(u),
            _ => None,
        }
    }

    /// Returns the inner `NewTypeStructSchema` if this is a `NewType` variant.
    pub fn as_newtype(&self) -> Option<&NewTypeStructSchema<'a>> {
        match self {
            VariantKind::NewType(n) => Some(n),
            _ => None,
        }
    }

    /// Returns the inner `TupleStructSchema` if this is a `Tuple` variant.
    pub fn as_tuple(&self) -> Option<&TupleStructSchema<'a>> {
        match self {
            VariantKind::Tuple(t) => Some(t),
            _ => None,
        }
    }

    /// Returns the inner `NamedStructSchema` if this is a `Named` variant.
    pub fn as_named(&self) -> Option<&NamedStructSchema<'a>> {
        match self {
            VariantKind::Named(n) => Some(n),
            _ => None,
        }
    }
}

impl<'a> EnumSchema<'a> {
    /// Contribute this enum to a schema hash (version-aware).
    ///
    /// Inactive variants (not yet introduced or removed) are excluded entirely.
    /// Active and deprecated variants are both included, but each is prefixed
    /// with a state byte: `0x00` for `Active`, `0x01` for `Deprecated`.
    /// This ensures the hash reflects deprecation transitions across versions.
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u64;
        build.update(&len.to_be_bytes());
        build.update(bytes);
        if let Some(states) = self.active_variants_at(version) {
            let included_count = states.iter().filter(|s| !s.is_inactive()).count() as u64;
            build.update(&included_count.to_be_bytes());
            for (i, state) in states.iter().enumerate() {
                if !state.is_inactive() {
                    build.update(&[if state.is_deprecated() {
                        0x01u8
                    } else {
                        0x00u8
                    }]);
                    self.variants[i].hash_build(version, build);
                }
            }
        } else {
            // No activation data for this version: include all variants as active.
            let cnt = self.variants.len() as u64;
            build.update(&cnt.to_be_bytes());
            for variant in self.variants {
                build.update(&[0x00u8]);
                variant.hash_build(version, build);
            }
        }
        self.version.hash_build(build);
    }
}

impl<'a> Schema<'a> {
    /// Compute a version-aware schema hash using blake3.
    ///
    /// For `Named` and `Enum` schemas, only fields/variants that are active
    /// at the given `version` are included in the hash, making the digest
    /// reflect the schema shape as seen at that protocol version.
    pub fn hash_build(&self, version: Version, build: &mut Hasher) {
        match self {
            Schema::Primitive(p) => p.hash_build(build),
            Schema::Option(inner) => {
                build.update(&[0x10]);
                inner.hash_build(version, build);
            }
            Schema::Seq(inner) => {
                build.update(&[0x11]);
                inner.hash_build(version, build);
            }
            Schema::Tuple(elems) => {
                build.update(&[0x12]);
                let len = elems.len() as u64;
                build.update(&len.to_be_bytes());
                for elem in *elems {
                    elem.hash_build(version, build);
                }
            }
            Schema::Map(k, v) => {
                build.update(&[0x13]);
                k.hash_build(version, build);
                v.hash_build(version, build);
            }
            Schema::Unit => {
                build.update(&[0x14]);
            }
            Schema::UnitStruct(u) => {
                build.update(&[0x15]);
                u.hash_build(build);
            }
            Schema::NewTypeStruct(n) => {
                build.update(&[0x16]);
                n.hash_build(version, build);
            }
            Schema::TupleStruct(t) => {
                build.update(&[0x17]);
                t.hash_build(version, build);
            }
            Schema::Named(s) => {
                build.update(&[0x18]);
                s.hash_build(version, build);
            }
            Schema::Enum(e) => {
                build.update(&[0x19]);
                e.hash_build(version, build);
            }
        }
    }

    /// Copy a schema value from a borrowed reference without changing any of
    /// its nested borrowed components.
    pub const fn from_ref(schema: &'a Schema<'a>) -> Self {
        match schema {
            Schema::Primitive(p) => Schema::Primitive(*p),
            Schema::Option(inner) => Schema::Option(inner),
            Schema::Unit => Schema::Unit,
            Schema::UnitStruct(unit) => Schema::UnitStruct(UnitStructSchema {
                name: unit.name,
                version: unit.version,
            }),
            Schema::NewTypeStruct(newtype) => Schema::NewTypeStruct(NewTypeStructSchema {
                name: newtype.name,
                inner: newtype.inner,
                version: newtype.version,
            }),
            Schema::TupleStruct(tuple) => Schema::TupleStruct(TupleStructSchema {
                name: tuple.name,
                elems: tuple.elems,
                version: tuple.version,
            }),
            Schema::Seq(inner) => Schema::Seq(inner),
            Schema::Tuple(elems) => Schema::Tuple(elems),
            Schema::Map(key, value) => Schema::Map(key, value),
            Schema::Named(schema) => Schema::Named(NamedStructSchema {
                name: schema.name,
                fields: schema.fields,
                index: schema.index,
                version: schema.version,
                field_activations: schema.field_activations,
            }),
            Schema::Enum(schema) => Schema::Enum(EnumSchema {
                name: schema.name,
                variants: schema.variants,
                version: schema.version,
                variant_activations: schema.variant_activations,
            }),
        }
    }

    /// Returns the inner `EnumSchema` if this is an `Enum` schema.
    pub fn as_enum(&self) -> Option<&EnumSchema<'a>> {
        match self {
            Schema::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Returns the inner `NamedStructSchema` if this is a `Named` schema.
    pub fn as_named(&self) -> Option<&NamedStructSchema<'a>> {
        match self {
            Schema::Named(n) => Some(n),
            _ => None,
        }
    }
}
