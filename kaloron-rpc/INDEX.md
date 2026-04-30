# kaloron-rpc

Version-aware RPC framework built on hyper. Supports HTTP/1.1, HTTP/2, and experimental HX (shared-memory) transports.

## Dependencies

- `tokio` — async runtime
- `anyhow` — error handling
- `blake3` — schema hashing
- `bytes` — buffer management
- `hyper` — HTTP client/server
- `http-body-util` — HTTP body utilities
- `typeid` — type-safe identifier
- `futures-util` — async streaming
- `kaloron` — core type-shape library
- `kaloron-codec` — textual codec
- `kaloron-rpc-macro` — service attribute macro
