# ClientChannel / ClientTransport Completion Logic Is Unsafe

Current `ClientChannel` and `ClientTransport` completion logic is insecure and needs refinement. This is a breaking change that requires careful design.

## Problem

- `ClientChannel::complete()` (hx Complete frame) must be called before the channel is dropped, otherwise the server receives an abrupt TCP close instead of a graceful shutdown.
- `Drop` impls for channel types do NOT send the Complete frame — they only abort the receiver task and break internal state.
- `ClientTransport::complete()` (which controls the underlying HTTP connection) also aborts the connection task, leaving no opportunity for the server to finish processing.
- There is no guarantee in the API that users will call `complete()` in the right order (channel first, then transport).
- H1 masks this issue because hyper's H1 handler detects the TCP close immediately and lets the server session terminate gracefully. H2 does not mask it — the H2 handler hangs when the TCP is dropped without GOAWAY.

## Impact

- H2 integration tests hang at shutdown unless the channel's `complete()` is called before dropping the transport.
- Users may silently leak connections or hang if they drop the client without a proper two-phase shutdown.

## Required Fixes (breaking)

1. Ensure `Drop` for `ServiceClient` / channel types calls `complete()` (not possible without async drop).
2. Restructure the `Client` / `ClientTransport` API so that the channel and transport phase their shutdown in a single async call, e.g. `Client::complete()` drains the channel first, then shuts down the transport.
3. Or decouple the HTTP connection from the channel lifetime so that the drop ordering is irrelevant.
