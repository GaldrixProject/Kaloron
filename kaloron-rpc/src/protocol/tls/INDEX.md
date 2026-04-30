# kaloron-rpc/src/protocol/tls

TLS transport implementation for HyperListener using rustls + tokio-rustls.

## Files

- `mod.rs` — TlsHyperListener wrapping tokio::net::TcpListener + rustls::ServerConfig

## Public API

- `TlsHyperListener` — implements `HyperListener<Io = TokioIo<TlsStream<TcpStream>>>`
  - `new(listener, config)` — wrap an existing tokio TcpListener with TLS config
  - `bind(addr, config)` — async-bind a new TcpListener with TLS config
  - Accept loop does TCP accept + TLS handshake, returns TLS-wrapped stream
  - Graceful shutdown via `complete()`
