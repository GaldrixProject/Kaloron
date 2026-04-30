# kaloron-codec/src

## Public Interface

- `TextualCodec` — concrete `Codec` implementation
- `TextReader`, `TextWriter` — low-level text format reader/writer
- `Codec` trait — abstract encode/decode interface
- `WriteExt`, `ReadExt` — I/O extension traits for facade creation
- `FdSend`, `FdRecv` (unix only) — file descriptor transport traits

## Module Architecture

```
lib.rs          Codec trait, TextualCodec, WriteExt, ReadExt, FdSend/FdRecv
└── textual/    Textual format implementation
    ├── mod.rs      High-level encode/decode/bound functions
    ├── text_reader.rs  Tokenizer/parser for text format
    ├── text_writer.rs  Renderer for text format
    └── visit/     Schema-driven visitor dispatch for text format
        ├── mod.rs
        ├── common.rs
        ├── primitives.rs
        ├── option.rs
        ├── sequence.rs
        ├── tuples.rs
        ├── maps.rs
        ├── named_struct.rs
        └── enums.rs
```
