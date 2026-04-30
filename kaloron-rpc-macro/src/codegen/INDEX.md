# kaloron-rpc-macro/src/codegen

Code generation modules for `#[kaloron_rpc]` attribute macro.

## Files

- `mod.rs` — Module root, re-export
- `attributes.rs` — `#[kaloron]` attribute codegen for method args and fields
- `arg_struct.rs` — Service method argument struct generation
- `client.rs` — Service client (`ServiceClient`) implementation generation
- `dispatch.rs` — `ServiceDispatch` implementation for server handler dispatch
- `method_schema.rs` — `MethodSchema` const generation per service method
- `service_factory.rs` — `ServiceFactory` impl and `SERVICE_SCHEMA` const generation
