# kaloron/src/visitor

Type visitation framework — the core of `TypeShape` derive dispatch.

## Files

- `mod.rs` — `TypeShape` trait definition, `SendVisitor`/`RecvVisitor` traits, visitor surface types, adapter factory functions
- `send_accept.rs` — `SendAccept*` adapter traits and `accept_send_*` factory functions for each visitor slot
- `recv_accept.rs` — `RecvAccept*` adapter traits and `accept_recv_*` factory functions for each visitor slot

## Architecture

The visitor slots (accept_primitive, accept_tuple, etc.) are split across send/recv variants for schema-driven codec dispatch. Generated `TypeShape` impls call into these adapters.
