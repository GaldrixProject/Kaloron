# kaloron-rpc/src/protocol/hx/reader

Frame reader — decodes inbound hx frames from an HTTP body stream.

## Files

- `mod.rs` — Re-exports FrameReader, InboundFrame
- `shell.rs` — FrameReader<C, H> public API, InboundFrame<C, H> header + data access
- `inner.rs` — FrameReaderInner: HTTP body accumulation, frame boundary detection
- `ring.rs` — RingSpread: byte ring buffer with scatter-read, FrameRead abstraction

## Data flow

`FrameReader::read_frame()` → `inner.read_frame_start()` → accumulate bytes from body stream.
`InboundFrame::header()` → codec-decoded header from frame prefix.
`InboundFrame::read_data<T>()` → codec-decoded body from remaining frame bytes via ring buffer.
