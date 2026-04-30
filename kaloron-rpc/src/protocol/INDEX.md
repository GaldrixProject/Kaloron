# kaloron-rpc/src/protocol

Transport protocol implementations for kaloron RPC.

## Submodules

- `hx/` — HTTP-framed hx protocol: H1/H2 client + server transports, frame reader/writer
- `tcp/` — `TcpHyperListener`: TCP transport implementing `HyperListener`
- `tls/` — `TlsHyperListener`: TLS transport implementing `HyperListener` (rustls + tokio-rustls)

## Re-exports (via `mod.rs`)

All public types from `hx`, `tcp`, and `tls` are re-exported at `crate::protocol` level.
