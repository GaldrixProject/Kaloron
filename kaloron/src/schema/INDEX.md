# kaloron/src/schema

Schema type system for kaloron's serialization framework.

## Files

- `mod.rs` — Core types: `Schema`, `Primitive`, `NamedStructSchema`, `EnumSchema`, `Variant`, `VariantKind`, `Field`, `ActivationState`, `hash_build`
- `version.rs` — `Version`, `VersionRange`, `DeprecationSpan`, `VersionActivationEntry` with ordering, activation queries, and hashing

## Dependencies

Depends on `blake3` for schema hash computation.
