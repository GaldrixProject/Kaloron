# kaloron-rpc/src/protocol/hx/writer

Frame writer — encodes outbound hx frames into an HTTP response body stream.

## Files

- `mod.rs` — Re-exports FrameWriter, FrameWriterBody
- `shell.rs` — FrameWriter<C, H> public API, FrameWriterBody (hyper Body impl), Drop → closed
- `inner.rs` — FrameWriterCore: facade, frame encoding, gater/builder coordination
- `ring.rs` — RingGather: byte ring buffer with gather-write, FrameBuilder for inline framing

## Data flow

`FrameWriter::frame(version, header, body)` → codec-encodes header + body into `RingGather`.
`FrameWriterBody` polls `RingGather` via hyper Body trait → sends data frames to transport.
`Drop` sets `closed = true` → body returns `None` → stream ends.
`FrameWriterBody::empty()` creates a permanently-closed body (for error responses).
