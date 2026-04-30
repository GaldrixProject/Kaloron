# kaloron-rpc/src/protocol/hx/server

HTTP server transport for kaloron hx protocol. Supports HTTP/1.1 and HTTP/2.

## Files

- `mod.rs` — Re-exports H1/H2 server config/transport/channel types
- `hx.rs` — HxServerService, HxServerChannel, HxServerConsts, serve loop, UnitJoinSet
- `h1.rs` — H1ServerConfig, H1ServerTransport, H1ServerChannel — HTTP/1.1 transport
- `h2.rs` — H2ServerConfig, H2ServerTransport, H2ServerChannel — HTTP/2 transport

## Architecture

`HxServerService` + `HxServiceFn` implement `hyper::service::Service<Request<Incoming>>`.
`HxServerConsts` holds shared config (services map, facades, endpoint, frame limit).
Each connection spawns a `serve()` future that drives the hx frame loop (reader ↔ writer ↔ dispatch).
`H1SessionBridge` / `H2SessionBridge` adapt `ServiceDispatch<T, HxServerChannel>` bounds.
