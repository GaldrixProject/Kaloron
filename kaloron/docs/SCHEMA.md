Kaloron Schema Design
=====================

Status: implemented

Overview
--------
This document describes the full versioned schema system in kaloron. It covers
schema types, version metadata, activation tables, explicit field/variant IDs,
hashing strategy, the `Gated<T>` type, runtime version validation, and the
derive macro integration.

The schema system has three layers:

1. **Schema metadata** — static structural descriptions of Rust types with
   version lifecycle data and explicit wire-stable IDs.
2. **Gated<T> wrapping** — fields that can be absent at some protocol version.
3. **Runtime validation** — `send`/`recv` enforce version constraints.

Byte buffers and fixed-size arrays are represented with sequence-shaped schemas (`Schema::Seq`), so `Vec<T>`, `Box<[T]>`, and `[T; N]` all share the same container shape.

Dependencies
------------
- **blake3** — used for schema hashing at runtime (version-aware digests).
- **kaloron-macro** — procedural derive macro that generates schema constants,
  activation tables, and send/recv implementations.

Schema types (kaloron/src/schema.rs)
-------------------------------------

### 1. Version type

Packed u64 representation of a semantic version:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u64);
```

Layout: `major (16 bits) | minor (16 bits) | patch (16 bits) | build (16 bits)`

Key methods:
- `Version::new(major, minor, patch, build)` — construct from components
- `Version::zero()` — the `0.0.0.0` version
- `Version::successor()` — next representable version in packed order
- `Version::hash_build(&self, build: &mut blake3::Hasher)` — tag `0xE0` + u64 BE

### 2. DeprecationSpan type

A single deprecation period with inclusive bounds:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DeprecationSpan {
    pub since: Version,
    pub until: Option<Version>,
}
```

- `DeprecationSpan::new(since)` — open-ended deprecation
- `DeprecationSpan::new_range(since, until)` — bounded deprecation
- `DeprecationSpan::contains(version)` — check if version falls within span
- `DeprecationSpan::hash_build(&self, build: &mut blake3::Hasher)` — tag `0xE1`

### 3. VersionRange type

Complete lifecycle for an item:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VersionRange {
    pub introduced: Version,
    pub deprecations: &'static [DeprecationSpan],
    pub removed_in: Option<Version>,
}
```

- `VersionRange::default()` — introduced at 0.0.0.0, no deprecations, not removed
- `VersionRange::introduced(v)` / `VersionRange::introduced_removed(intro, removed)`
- `VersionRange::exists_at(version)` — true if introduced and not yet removed
- `VersionRange::is_deprecated_at(version)` — true if any deprecation span covers version
- `VersionRange::hash_build(&self, build: &mut blake3::Hasher)` — tag `0xF0`

### 4. ActivationState enum

A three-state marker for fields and variants at a given version:

```rust
pub enum ActivationState {
    Active,      // exists and not deprecated
    Deprecated,  // exists but deprecated
    Inactive,    // does not exist at this version
}
```

### 5. VersionActivationEntry type

Maps a starting version to which fields/variants are active:

```rust
pub struct VersionActivationEntry<'a> {
    pub version: Version,
    pub active: &'a [ActivationState],
}
```

A private `binary_search_activation` helper provides O(log n) lookup into
activation tables.

### 6. Schema node structures

Each schema node carries a `version: VersionRange` field. `Field` and
`Variant` additionally carry an explicit `id: u32` for wire stability.

```rust
pub struct Field<'a> {
    pub id: u32,
    pub name: &'a str,
    pub ty: &'a Schema<'a>,
    pub version: VersionRange,
}

pub struct UnitStructSchema<'a> {
    pub name: &'a str,
    pub version: VersionRange,
}

pub struct NewTypeStructSchema<'a> {
    pub name: &'a str,
    pub inner: &'a Schema<'a>,
    pub version: VersionRange,
}

pub struct TupleStructSchema<'a> {
    pub name: &'a str,
    pub elems: &'a [&'a Schema<'a>],
    pub version: VersionRange,
}

pub struct NamedStructSchema<'a> {
    pub name: &'a str,
    pub fields: &'a [Field<'a>],
    index: &'a [(&'a str, usize)],
    pub version: VersionRange,
    pub field_activations: &'a [VersionActivationEntry<'a>],
}

pub struct EnumSchema<'a> {
    pub name: &'a str,
    pub variants: &'a [Variant<'a>],
    pub version: VersionRange,
    pub variant_activations: &'a [VersionActivationEntry<'a>],
}
```

### 7. Variant is a struct + VariantKind enum

```rust
pub struct Variant<'a> {
    pub id: u32,
    pub kind: VariantKind<'a>,
}

pub enum VariantKind<'a> {
    Unit(UnitStructSchema<'a>),
    NewType(NewTypeStructSchema<'a>),
    Tuple(TupleStructSchema<'a>),
    Named(NamedStructSchema<'a>),
}
```

### 8. Top-level Schema enum

```rust
pub enum Schema<'a> {
    Primitive(Primitive),
    Option(&'a Schema<'a>),
    Seq(&'a Schema<'a>),
    Tuple(&'a [&'a Schema<'a>]),
    Map(&'a Schema<'a>, &'a Schema<'a>),
    Unit,
    UnitStruct(UnitStructSchema<'a>),
    NewTypeStruct(NewTypeStructSchema<'a>),
    TupleStruct(TupleStructSchema<'a>),
    Named(NamedStructSchema<'a>),
    Enum(EnumSchema<'a>),
}
```

Hashing (blake3, version-aware)
-------------------------------
Schema hashing uses **blake3** at runtime. All `hash_build` methods take a
mutable reference to a `blake3::Hasher`. Types that contain version-filtered
children accept a `version: Version` parameter.

### Signature patterns

- **No version parameter** (leaf/metadata types):
  - `Primitive::hash_build(&self, build: &mut Hasher)`
  - `Version::hash_build(&self, build: &mut Hasher)`
  - `DeprecationSpan::hash_build(&self, build: &mut Hasher)`
  - `VersionRange::hash_build(&self, build: &mut Hasher)`
  - `UnitStructSchema::hash_build(&self, build: &mut Hasher)`

- **Version parameter** (propagating/filtering types):
  - `NewTypeStructSchema::hash_build(&self, version: Version, build: &mut Hasher)`
  - `TupleStructSchema::hash_build(&self, version: Version, build: &mut Hasher)`
  - `Field::hash_build(&self, version: Version, build: &mut Hasher)`
  - `NamedStructSchema::hash_build(&self, version: Version, build: &mut Hasher)` — **filters by activation table**
  - `Variant::hash_build(&self, version: Version, build: &mut Hasher)`
  - `VariantKind::hash_build(&self, version: Version, build: &mut Hasher)`
  - `EnumSchema::hash_build(&self, version: Version, build: &mut Hasher)` — **filters by activation table**
  - `Schema::hash_build(&self, version: Version, build: &mut Hasher)`

### Version-aware filtering

For `NamedStructSchema` and `EnumSchema`, the `hash_build` method uses the
activation table to determine which fields/variants are present at the given
version:

- **Inactive** items (not yet introduced, or removed) are **excluded** from
  the hash entirely.
- **Active** and **Deprecated** items are both **included**, but each is
  preceded by a one-byte state marker:
  - `0x00` — item is `Active`
  - `0x01` — item is `Deprecated`

This means the hash of a type changes whenever a field or variant:
- first appears (introduced),
- enters deprecation,
- exits a bounded deprecation period back to active,
- or is removed.

Two peers hashing the same schema at the same version will always agree,
while hashing at different versions (even if the set of fields hasn't changed
but a deprecation boundary was crossed) will produce distinct digests.

When no activation data covers the given version (the version predates all
activation entries, or the activation table is empty), all items are included
as if they were `Active` (state byte `0x00`).

### Tag bytes

| Type / Kind              | Tag  |
|--------------------------|------|
| `Option`                 | 0x10 |
| `Seq`                    | 0x11 |
| `Tuple`                  | 0x12 |
| `Map`                    | 0x13 |
| `Unit`                   | 0x14 |
| `UnitStruct`             | 0x15 |
| `NewTypeStruct`          | 0x16 |
| `TupleStruct`            | 0x17 |
| `Named`                  | 0x18 |
| `Enum`                   | 0x19 |
| `Primitive::Bool`        | 0x20 |
| `Primitive::I8..U128`    | 0x21–0x2A |
| `Primitive::F32`         | 0x2B |
| `Primitive::F64`         | 0x2C |
| `Primitive::Char`        | 0x2D |
| `Primitive::String`      | 0x2E |
| `Primitive::File`        | 0x2F | Unix-like only |
| Byte buffers / arrays    | represented as `Schema::Seq` |
| `VariantKind::Unit`      | 0x30 |
| `VariantKind::NewType`   | 0x31 |
| `VariantKind::Tuple`     | 0x32 |
| `VariantKind::Named`     | 0x33 |
| `Version`                | 0xE0 |
| `DeprecationSpan`        | 0xE1 |
| `VersionRange`           | 0xF0 |

Attribute syntax
----------------
All attributes use the `#[kaloron(...)]` namespace.

### ID (required on named struct fields and enum variants)

```rust
#[kaloron(id = 0)]
```

IDs must form a dense range `[0, N)` within their scope.

### Introduction (required on types)

```rust
#[kaloron(introduced = "1.2.3")]
#[kaloron(introduced = "1.2.3.4")]
```

### Deprecation (optional, repeatable)

```rust
#[kaloron(deprecated = "2.0.0")]
#[kaloron(deprecated = "2.0.0..2.1.0")]
```

### Removal (optional)

```rust
#[kaloron(removed = "3.0.0")]
```

### Combined attributes

```rust
#[kaloron(id = 0, introduced = "1.0.0")]
#[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
```

### Full example

```rust
#[derive(TypeShape)]
#[kaloron(introduced = "1.0.0")]
struct Config {
    #[kaloron(id = 0)]
    base: u32,

    #[kaloron(id = 1, introduced = "2.0.0")]
    new_feature: Gated<String>,

    #[kaloron(id = 2, introduced = "1.5.0", removed = "3.0.0")]
    temporary: Gated<bool>,
}

#[derive(TypeShape)]
#[kaloron(introduced = "1.0.0")]
enum Message {
    #[kaloron(id = 0)]
    Ping,

    #[kaloron(id = 1)]
    Data {
        #[kaloron(id = 0)]
        payload: Vec<u8>,
    },
}
```

Inheritance model
-----------------
Inner items automatically inherit version metadata from enclosing items.
`id` is never inherited.

1. **Introduced**: inherits if not specified on inner item.
2. **Deprecation**: inherits if no `deprecated` attrs on inner item.
3. **Removal**: inherits if not specified on inner item.
4. **Override**: explicit attrs on inner items completely override inherited values.

Gated<T> type (kaloron/src/gated.rs)
--------------------------------------

`Gated<T>` wraps fields that can be absent at some protocol version.

```rust
pub enum Gated<T> {
    Active(T),
    Void,
}
```

### Usage rules

1. A field **can be void** if its effective version range doesn't fully cover
   the container's version range.
2. Fields that can be void **MUST** use `Gated<T>` as their Rust type.
3. Fields that are always active **MUST NOT** use `Gated<T>`.
4. In the schema, `Gated<T>` is transparent — the field records `T::SCHEMA`.
5. The derive macro enforces these rules at compile time.

Runtime version validation
--------------------------

### Named struct fields

**Send**: Validates gated fields against the version:
- Inactive field → must be `Void` (error if `Active`)
- Deprecated field → either `Active` or `Void` is acceptable
- Active field → must be `Active` (error if `Void`)

**Recv**: After receiving fields, sets inactive gated fields to `Void`.

### Enum variants

**Send**: Validates the current variant exists at the version (error if inactive).

**Recv**: Validates the received variant index exists at the version.

### Named variant fields

Named variant fields follow the same rules as named struct fields, with the
variant's version range acting as the container for its fields.

Compile-time validation
-----------------------
The derive macro validates at compile time:

1. Version format (3- or 4-component, components 0–65535).
2. Version ordering (introduced ≤ removed, introduced ≤ deprecation.since, etc.).
3. No overlapping deprecation spans.
4. `introduced` required on types.
5. `id` required on named struct fields and enum variants.
6. IDs form a dense `[0, N)` range.
7. Void-able fields must use `Gated<T>`; always-active fields must not.

Macro implementation
--------------------

### VersionAttrs (kaloron-macro/src/version.rs)

Central parsed representation:

```rust
pub(crate) struct VersionAttrs {
    pub introduced: Option<Version>,
    pub deprecations: Vec<DeprecationSpan>,
    pub removed_in: Option<Version>,
    pub id: Option<u32>,
}
```

### Activation table generation

`compute_activation_table_tokens(item_versions)` collects all "change point"
versions into a `BTreeSet`, then for each change point computes a three-state
activation array. Returns token stream for `&[VersionActivationEntry { ... }, ...]`.

### Code generation

- Schema constants: fields and variants ordered by id.
- Send methods: bind fields, construct `__Tv` tree, optionally validate gated fields.
- Recv methods: build with `__Tb` tree, optionally fixup gated fields via activation table.
- Activation tables: computed from inherited versions in id order.
