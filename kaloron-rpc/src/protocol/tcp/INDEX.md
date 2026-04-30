# kaloron-rpc/src/protocol/tcp

TCP transport implementation for HyperListener and HyperStream.

## Files

- `mod.rs` — TcpHyperListener wrapping tokio::net::TcpListener

## Public API

- `TcpHyperListener` — implements `HyperListener<Io = TokioIo<TcpStream>>`
  - `new(listener)` — wrap an existing tokio TcpListener
  - `bind(addr)` — async-bind a new TcpListener
  - Accept loop supports graceful shutdown via `complete()`
