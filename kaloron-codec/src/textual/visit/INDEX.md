# kaloron-codec/src/textual/visit

Schema-driven visitor dispatch for the textual codec format.

## Files

- `mod.rs` — Module root, re-exports
- `common.rs` — Shared visitor utilities
- `primitives.rs` — Primitive type encode/decode dispatch
- `option.rs` — `Option<T>` encode/decode dispatch
- `sequence.rs` — Sequence/array encode/decode dispatch
- `tuples.rs` — Tuple encode/decode dispatch
- `maps.rs` — Map encode/decode dispatch
- `named_struct.rs` — Named struct encode/decode dispatch
- `enums.rs` — Enum encode/decode dispatch
