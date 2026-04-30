# Kaloron

Version-gated serialization framework for Rust.

Kaloron is a codec-agnostic serialization framework where every type carries a
schema that tracks when each field, variant, or method was introduced,
deprecated, or removed. This enables forward/backward compatibility across
versioned service boundaries.

## Workspace Crates

| Crate | Description |
|---|---|
| [`kaloron`](kaloron) | Core library: `TypeShape` derive macro, schema types, versioning, visitor framework |
| [`kaloron-codec`](kaloron-codec) | Codec implementations (textual format) |
| [`kaloron-macro`](kaloron-macro) | `#[derive(TypeShape)]` proc-macro implementation |
| [`kaloron-rpc`](kaloron-rpc) | RPC framework: H1/H2 client + server transport, hx protocol |
| [`kaloron-rpc-macro`](kaloron-rpc-macro) | `#[kaloron_rpc]` proc-macro implementation |

## Build

```bash
cargo build
cargo test
```

## License

MIT — see [LICENSE](LICENSE).
