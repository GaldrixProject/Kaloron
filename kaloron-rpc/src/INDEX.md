# kaloron-rpc/src

## Public Interface

- `InFlightCallIds` — call ID allocation and duplicate detection
- `ClientChannel`, `ServerChannel` — channel abstractions
- `ClientTransport`, `ServerTransport` — transport abstractions
- `RpcClient`, `RpcServer` — high-level client/server
- `KaloronRpcError` — error types
- `EffectiveSchema`, `effective_schema_hash` — schema utilities
- `protocol::hx` — H1/HX transport protocol implementations

## Module Architecture

```
lib.rs          Re-exports all public API, re-exports kaloron_rpc macro
├── call.rs         InFlightCallIds
├── channel.rs      ClientChannel, ServerChannel traits
├── client.rs       RpcClient
├── error.rs        KaloronRpcError
├── schema.rs       EffectiveSchema, effective_schema_hash
├── server.rs       RpcServer
├── transport.rs    ClientTransport, ServerTransport traits
├── utility.rs      Version negotiation utilities
    └── protocol/       Transport protocol implementations
        └── mod.rs
        └── hx/          H1 transport and HX framing
            ├── mod.rs
            ├── INDEX.md
            ├── client/      Client-side transport and channel
            │   ├── mod.rs
            │   ├── h1.rs        H1ClientConfig, H1ClientTransport
            │   └── hx.rs        HxClientChannel, H1ClientSession
            ├── server/      Server-side transport and channel
            │   ├── mod.rs
            │   ├── h1.rs        H1ServerConfig, H1ServerTransport
            │   └── hx.rs        OpaqueHxServerChannel, serve()
            ├── facade.rs      Facade/FacadeBuilder
            ├── message.rs     Frame header types
            ├── reader/        Async frame reader with ring buffer
            │   ├── mod.rs
            │   ├── ring.rs         RingSpread, Frame, FrameRead
            │   ├── inner.rs        FrameReaderInner, InboundFrameInner
            │   └── shell.rs        FrameReader<C,H>, InboundFrame<C,H>
            ├── writer/        Async frame writer with ring buffer
            │   ├── mod.rs
            │   ├── ring.rs         RingGather, FrameBuilder
            │   ├── inner.rs        FrameWriterCore, OutboundFrameInner
            │   └── shell.rs        FrameWriter<C,H>, FrameWriterBody
            └── tests.rs
```
