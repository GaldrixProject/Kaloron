# Kaloron RPC Design

Status: channel trait surface defined and macro-generated; concrete transport
adapters and typed payload codecs are still in progress.

This document describes the intended transport model for `kaloron-rpc` and the
runtime pieces that already exist in the workspace.  The design is intentionally
biased toward local IPC and modern multiplexed transports rather than a single
long-lived monolithic channel.

---

## Design priorities

1. **One service client/provider pair per service stream**
   - Opening a client for a service opens a **new stream dedicated to that service**.
   - This fits local deployments that can create anonymous Unix-domain socket pairs,
     and remote deployments built on multiplexed transports such as **HTTP/2** or
     **HTTP/3**.
   - Because each stream is bound to exactly one service, calls for unrelated
     services never interfere with one another.

2. **Handshake is completely out of band**
   - The handshake does **not** consume bytes on the service stream itself.
   - Before the service stream is considered established, the client sends:
     - the requested payload protocol,
     - the service identifier,
     - the requested version, and
     - the **BLAKE3 hash of the effective service schema** at that version.
   - The server computes the same effective schema hash and accepts the stream
     only when all four fields match exactly.

3. **Multiple payload protocols are supported**
   - The framework supports at least a **binary** payload format for normal
     traffic and a **textual** payload format for debugging and diagnostics.
   - Protocol selection is part of the out-of-band handshake, not per-request.

4. **Request correlation uses `u64` call IDs**
   - Every in-flight request on a service stream has a unique unsigned 64-bit
     call ID.  `InFlightCallIds` provides reusable client/server call-ID tracking.

5. **Service-stream framing is always WebSocket framing**
   - Regardless of the underlying transport, the bytes on the service stream are
     framed using WebSocket frames.

---

## Transport model

### Stream lifecycle

```text
client wants service S
    │
    ├─ out-of-band handshake request
    │    { protocol, service, version, effective_schema_hash }
    │
    ├─ server validates request against local service schema
    │
    └─ if accepted, both sides bind one new stream to service S
         and exchange only service-local requests/responses on it
```

### Why the stream is service-bound

- per-request service identifiers are unnecessary,
- routing is simpler,
- unrelated services cannot head-of-line block each other, and
- transport adapters can map naturally onto UDS pairs or multiplexed substreams.

---

## Handshake contract

### Effective schema hash

The handshake hash is computed from the **effective surface visible at the
requested version**: methods not yet introduced or already removed are excluded,
deprecated-but-still-active items remain included.  This makes the handshake a
strict compatibility check.

### Server-side validation order

1. service identifier matches a known service,
2. requested payload protocol is supported,
3. requested version is valid for that service,
4. computed effective schema hash matches the client-supplied hash exactly.

---

## Current runtime API

### Service-trait level (`#[kaloron_rpc]` macro)

`#[kaloron_rpc("ServiceName", introduced = "1.0.0")]` on a trait expands to:

- The **modified service trait** — `#[kaloron]` attributes are stripped; each
  `async fn` becomes `fn method(&self, …) -> impl Future<Output = RpcResult<T>> + Send + '_`.
  `Send + Sync` are added as supertraits. The macro no longer wraps the return
  type for you: the outer `RpcResult` is the RPC/transport layer, and any
  application-specific failure mode should live in the inner `T` (for example,
  `RpcResult<Result<Ok, DomainError>>`).

- **`<Name>Factory`** — a factory marker struct that implements `ServiceFactory`.

- **`<Name>Client<TT>`** — a type alias for `ServiceClient<TT, <Name>Factory>`.  The
  generated `impl<TT: ClientTransport> <Name>Service for ServiceClient<TT, <Name>Factory>`
  packs arguments, calls `ClientChannel::call` on the underlying transport
  channel, and maps transport failures into `RpcError::TransportError`.

- **`impl<H, TC> ServiceDispatch<H, TC> for <Name>Factory`** — implements the
  server-side dispatch.  `dispatch` handles exactly **one call** at a time:
  it reads the typed request from the channel, calls the handler, and writes the
  typed response (or a fault) back.  The containing message loop is owned by
  `Server::run`.

### Channel traits (`channel.rs`)

#### `ServerChannel`

```rust
pub trait ServerChannel {
    fn wait(&mut self)
        -> impl Future<Output = anyhow::Result<Option<u32>>> + Send + '_;

    fn read<T: TypeShape>(&mut self)
        -> impl Future<Output = anyhow::Result<Option<MessageRequest<T>>>> + Send + '_;

    fn write<T: TypeShape>(&mut self, m: MessageResponse<T>)
        -> impl Future<Output = anyhow::Result<()>> + Send + '_;

    fn read_fault(&mut self)
        -> impl Future<Output = anyhow::Result<MessageRequest<()>>> + Send + '_;

    fn write_fault(&mut self, m: MessageResponse<()>)
        -> impl Future<Output = anyhow::Result<()>> + Send + '_;

    fn complete(&mut self)
        -> impl Future<Output = anyhow::Result<()>> + Send + '_;
}
```

#### `ServiceDispatch<TH, TC>`

```rust
pub trait ServiceDispatch<TH, TC: ServerChannel>: ServiceFactory {
    fn dispatch<'a>(
        handler: &'a TH,
        method_id: u32,
        channel: &'a mut TC,
    ) -> impl Future<Output = bool> + Send + 'a;
}
```

`dispatch` handles a **single call**:
- `true` — call was processed; the message loop should continue.
- `false` — a transport error occurred; the connection should be closed.

Only the outer `Err(RpcError)` returned by the handler is treated as an RPC
fault and translated into a fault response.  If the operation needs its own
application-level failure mode, encode that as the inner `T` payload inside
`RpcResult<T>` so it remains distinct from the RPC layer.

#### `ServerTransport` and `Server`

```rust
pub trait ServerTransport: Sized {
    type Config;
    type Channel: ServerChannel;

    async fn configure(config: Self::Config) -> anyhow::Result<Self>;
    async fn accept(
        &mut self,
        v: &impl ServiceVerify,
    ) -> anyhow::Result<Option<AcceptedChannel<Self::Channel>>>;
    async fn complete(&mut self) -> anyhow::Result<()>;
}
```

`ServerBuilder<TT>` registers services and builds a `Server<TT>`:

```rust
let server = ServerBuilder::<MyTransport>::new(config)
    .serve::<MyHandler, MyHandlerFactory>(|| MyHandler::new())
    .build()
    .await?;

server.run().await?;
```

`Server::run()` drives the **server instance listener loop** (calls
`accept` in a loop, routing accepted channels to per-service dispatch tasks
via mpsc channels) and the **per-connection message loop** (calls
`ServiceDispatch::dispatch` for each incoming call until the connection closes
or a transport error occurs).  Because `ServiceDispatch::dispatch` returns a
`Send` future, all connection sub-tasks run on the multi-threaded Tokio
runtime via `tokio::spawn`.

### Client side

```rust
pub trait ClientChannel {
    fn call<TP: TypeShape, TR: TypeShape>(
        &self,
        method_id: u32,
        parameters: TP,
    ) -> impl Future<Output = anyhow::Result<RpcResult<TR>>> + Send + '_;

    fn complete(&mut self)
        -> impl Future<Output = anyhow::Result<()>> + Send + '_;
}

pub trait ClientTransport: Sized {
    type Config;
    type Channel: ClientChannel;

    fn configure(config: Self::Config)
        -> impl Future<Output = anyhow::Result<Self>> + Send;
    fn connect(
        &mut self,
        id: String,
        version: Version,
        hash: &[u8; 32],
    ) -> impl Future<Output = anyhow::Result<Self::Channel>> + Send + '_;
    fn complete(&mut self)
        -> impl Future<Output = anyhow::Result<()>> + Send + '_;
}
```

`Client<TT>` owns the transport and opens a service-specific channel via
`ClientTransport::connect`.  The macro-generated `<Name>Client<TT>` alias is a
`ServiceClient<TT, <Name>Factory>`, and the generated service trait impl uses
`ServiceClient::channel()` to reach the underlying `ClientChannel`.
Transport-level failures are converted into `RpcError::TransportError`, while
RPC-level faults continue to flow through the outer `RpcResult` layer.

---

## Implementing a handler

```rust
struct MyHandler { /* … */ }

impl MyService for MyHandler {
    async fn my_method(&self, arg: u64) -> RpcResult<String> {
        Ok(format!("result for {arg}"))
    }
}
```

Because the service trait requires `fn -> impl Future + Send + '_`, the
`async fn` implementation's body must produce a `Send` future.  In practice
this means:

- `RpcResult<T>` is the outer RPC/result layer.
- If the operation has its own domain error, make `T` a `Result<Ok, DomainError>`.
- RPC/transport failures should use `Err(RpcError)` and remain separate from
  the inner application result.
- `MyHandler: Send + Sync` (required by the service trait's `Send + Sync`
  supertrait bounds).
- The async body must not hold `!Send` values across `.await` points.

For most straightforward handlers (no `Rc`, no `Cell`, no non-`Send` locks)
this is satisfied automatically.

---

## Still intentionally incomplete

- Concrete `ClientTransport` / `ServerTransport` implementations (UDS, h2, h3…).
- Typed payload encode/decode behind `ServerChannel` read/write.
- A fully specified text payload format.
- Transport-level WebSocket frame helpers live in `kaloron-rpc::protocol::ws`, with HTTP/1 upgrade negotiation in `kaloron-rpc::protocol::ws1`.
- End-to-end stream establishment over real transports.
