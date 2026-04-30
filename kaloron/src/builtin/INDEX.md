# kaloron/src/builtin

`TypeShape` implementations for Rust's built-in types.

## Files

- `mod.rs` — Module root, re-exports
- `primitives.rs` — `bool`, `i/u{8,16,32,64,128}`, `f{32,64}`, `char`, `str`
- `tuples.rs` — Tuple impls (unit through 32 elements)
- `sequences.rs` — `Vec<T>`, `Box<[T]>`, arrays `[T; N]`
- `option.rs` — `Option<T>`
- `result.rs` — `Result<T, E>`
- `maps.rs` — `HashMap<K, V>`, `BTreeMap<K, V>`
