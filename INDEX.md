# Kaloron Workspace

Schema-first type-shape library for Rust with version-aware serialization, textual codec, and RPC framework.

## Crates

- **kaloron**: Core library. Defines `TypeShape`, `Schema`, `Version`, visitor traits, and builtin type implementations.
- **kaloron-macro**: Derive macro `#[derive(TypeShape)]` for automatic schema generation.
- **kaloron-codec**: Textual codec (JSON-like format) for encoding/decoding `TypeShape` values.
- **kaloron-rpc**: RPC framework built on hyper (HTTP/1.1, HTTP/2). Version-aware service definitions with H1, H2, and experimental HX transports.
- **kaloron-rpc-macro**: Attribute macro `#[kaloron_rpc]` for transforming service traits into client/server adapter pairs.

## Dependencies

- kaloron → kaloron-macro
- kaloron-codec → kaloron
- kaloron-rpc → kaloron, kaloron-codec, kaloron-rpc-macro
