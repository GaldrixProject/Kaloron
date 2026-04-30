# kaloron

Core type-shape library providing schema definitions, visitor traits, and version-aware serialization primitives.

## Dependencies

- `anyhow` — error handling
- `blake3` — schema hashing
- `seq-macro` — compile-time sequence generation for tuple impls
- `kaloron-macro` — proc-macro for `#[derive(TypeShape)]`
