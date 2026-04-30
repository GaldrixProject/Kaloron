# kaloron-rpc/src/protocol/hx/client

HTTP client transport for kaloron hx protocol. Supports HTTP/1.1 and HTTP/2.

## Files

- `mod.rs` — Re-exports H1/H2 client config/transport/channel types
- `hx.rs` — HxClientRequester, HxClientChannel, HxClientChannelBuilder — protocol logic
- `h1.rs` — H1ClientConfig, H1ClientTransport, H1ClientChannel — HTTP/1.1 transport
- `h2.rs` — H2ClientConfig, H2ClientTransport, H2ClientChannel — HTTP/2 transport

## Architecture

`HxClientRequester` trait abstracts the HTTP request/response handoff.
Each transport (h1/h2) implements a `HxClientRequester` backed by hyper's `SendRequest`.
`HxClientChannelBuilder` wraps the requester into a `ClientChannel` implementation.
