# kaloron/src

## Public Interface (re-exported via lib.rs)

- **schema** — `Schema`, `Primitive`, `Version`, `VersionRange`, `DeprecationSpan`, `ActivationState`, `VersionActivationEntry`, `NamedStructSchema`, `EnumSchema`, `Variant`, `VariantKind`, `Field`, and related structs
- **visitor** — `TypeShape` trait, `SendVisitor`/`RecvVisitor` traits, `SendAccept`/`RecvAccept` dispatch traits, all `*Send`/`*Recv` visitor surfaces, and adapter factory functions
- **gated** — `Gated<T>` version-gated value enum
- **marker** — Marker traits for compile-time type-level classification
- **utility** — Internal composite field builders (`__TbN`, `__TvN`, `__I16`, `__TbCp`, `__TvCp`) for generated code
- **builtin** — `TypeShape` implementations for primitives, tuples, sequences, Option, Result, and maps

## Module Architecture

```
lib.rs              Re-exports all public API
├── schema/         Schema definitions
│   ├── mod.rs      Core types: Schema, Primitive, struct/enum schemas, hash_build
│   └── version.rs  Version, DeprecationSpan, VersionRange, ActivationState
├── visitor/        Visitor traits and adapter factories
│   ├── mod.rs      Core visitor traits (SendVisitor, RecvVisitor, etc.), TypeShape
│   ├── send_accept.rs  SendAccept* adapter traits and factory functions
│   └── recv_accept.rs  RecvAccept* adapter traits and factory functions
├── gated.rs        Gated<T> version-gated value
├── marker.rs       Marker trait definitions
├── utility.rs      Internal field builder types for generated code
└── builtin/        Builtin TypeShape impls
    ├── mod.rs
    ├── primitives.rs
    ├── tuples.rs
    ├── sequences.rs
    ├── option.rs
    ├── result.rs
    └── maps.rs
```
