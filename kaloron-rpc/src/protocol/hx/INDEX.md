# kaloron-rpc/src/protocol/hx

kaloron hx (HTTP-framed) protocol implementation: codec-agnostic frame reader/writer, client transport, server transport.

## Module structure

```
hx/
├── consts.rs    Protocol constants (header names, textual protocol id)
├── mod.rs       Re-exports: client/server transports, facade, traits
├── traits.rs    HyperStream, HyperListener traits
├── utils.rs     HeaderPair helper struct
├── facade.rs    Facade / FacadeBuilder — type-erased shared service map
├── message.rs   HxClientBoundHeader, HxServerBoundHeader frame headers
├── client/      Client transport: H1Client*, H2Client*, HxClientChannel/Requester
├── server/      Server transport: H1Server*, H2Server*, HxServerChannel/Service
├── reader/      FrameReader, InboundFrame — streaming frame decode via RingSpread
└── writer/      FrameWriter, FrameWriterBody — streaming frame encode via RingGather
```

## Public API (re-exported at `crate::protocol`)

- `HyperStream`, `HyperListener` — traits for I/O and accept loop
- `Facade`, `FacadeBuilder` — shared service injection
- `H1ClientChannel`, `H1ClientConfig`, `H1ClientTransport` — HTTP/1.1 client
- `H2ClientChannel`, `H2ClientConfig`, `H2ClientTransport` — HTTP/2 client
- `H1ServerChannel`, `H1ServerConfig`, `H1ServerTransport` — HTTP/1.1 server
- `H2ServerChannel`, `H2ServerConfig`, `H2ServerTransport` — HTTP/2 server
