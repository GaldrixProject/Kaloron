# kaloron-macro/src

## Public Interface

- `#[proc_macro_derive(TypeShape)]` — entry point for `derive(TypeShape)`

## Module Architecture

```
lib.rs          Proc-macro entry point
├── root_ir.rs      Root IR: parses DeriveInput into typed intermediate representation
├── version.rs      Version attribute parsing, validation, activation table generation
├── struct_ir/      Struct codegen dispatch
│   ├── mod.rs
│   ├── named_struct_ir.rs
│   ├── tuple_struct_ir.rs
│   ├── newtype_struct_ir.rs
│   └── unit_struct_ir.rs
├── enum_ir/        Enum codegen dispatch
│   ├── mod.rs
│   ├── named_variant_ir.rs
│   ├── tuple_variant_ir.rs
│   ├── newtype_variant_ir.rs
│   └── unit_variant_ir.rs
└── common/         Shared codegen utilities
    ├── mod.rs
    ├── container.rs
    ├── fields.rs
    ├── send_builder.rs
    └── recv_builder.rs
```
