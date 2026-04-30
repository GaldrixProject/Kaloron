# kaloron-rpc-macro/src

## Public Interface

- `#[proc_macro_attribute]` — `kaloron_rpc` attribute macro entry point

## Module Architecture

```
lib.rs          Proc-macro attribute entry point, parse_service_meta
├── ir.rs           Internal representation of parsed service trait
├── parse.rs        Parsing service trait items into IR
├── validate.rs     Validation of service IR constraints
├── version.rs      Version attribute parsing for RPC services
├── expand.rs       Top-level codegen expansion dispatch
└── codegen/        Code generation modules
    ├── mod.rs
    ├── attributes.rs
    ├── arg_struct.rs
    ├── client.rs
    ├── dispatch.rs
    ├── method_schema.rs
    └── service_factory.rs
```
