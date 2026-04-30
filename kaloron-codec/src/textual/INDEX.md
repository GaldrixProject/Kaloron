# kaloron-codec/src/textual

Textual codec format — hx wire format encoding/decoding.

## Files

- `mod.rs` — High-level `encode`, `decode`, `bound` functions; bound calculation helpers
- `text_reader.rs` — `TextReader`: tokenizer/parser for text format input
- `text_writer.rs` — `TextWriter`: renderer for text format output
- `visit/` — Schema-driven visitor dispatch for text encoding/decoding
