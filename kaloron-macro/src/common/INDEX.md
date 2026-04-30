# kaloron-macro/src/common

Shared codegen utilities for `derive(TypeShape)`.

## Files

- `mod.rs` — Module root, re-exports
- `container.rs` — Container type codegen dispatch
- `fields.rs` — Field-level codegen: named/tuple field attributes, version gating
- `send_builder.rs` — `SendBuilder`: builds send-side visitor expression from field layout
- `recv_builder.rs` — `RecvBuilder`: builds recv-side visitor expression from field layout
