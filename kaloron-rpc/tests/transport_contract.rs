// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Transport-contract tests for `kaloron-rpc`.
//!
//! Tests here verify the schema-level invariants that are independent of the
//! transport implementation: effective schema hashing and call-ID tracking.
//! Tests that relied on the old `handshake_request` / `validate_handshake` /
//! `RpcProtocol` / `wire` API surface have been removed; those parts of the
//! protocol are still to-be-implemented in concrete transport adapters.

use kaloron::Version;
use kaloron_rpc::{InFlightCallIds, RpcError, ServiceFactory, kaloron_rpc};

#[kaloron_rpc("DebugService", introduced = "1.0.0")]
trait DebugService {
    #[kaloron(id = 0x0001)]
    async fn ping(&self, #[kaloron(id = 0)] value: u64) -> kaloron_rpc::RpcResult<u64>;

    #[kaloron(id = 0x0002, introduced = "1.1.0")]
    async fn dump_state(&self) -> kaloron_rpc::RpcResult<String>;
}

// ---------------------------------------------------------------------------
// Schema hash invariants
// ---------------------------------------------------------------------------

#[test]
fn test_effective_schema_hash_changes_with_service_surface() {
    let v1_0_0 = Version::new(1, 0, 0, 0);
    let v1_1_0 = Version::new(1, 1, 0, 0);

    assert_ne!(
        DebugServiceFactory::effective_schema_hash(v1_0_0),
        DebugServiceFactory::effective_schema_hash(v1_1_0),
        "adding `dump_state` in v1.1.0 must produce a different schema hash",
    );
}

// ---------------------------------------------------------------------------
// InFlightCallIds
// ---------------------------------------------------------------------------

#[test]
fn test_in_flight_call_ids_reuse_first_available_identifier() {
    let mut ids = InFlightCallIds::new();

    let first = ids.allocate().expect("first call id");
    let second = ids.allocate().expect("second call id");
    let third = ids.allocate().expect("third call id");

    assert_eq!((first, second, third), (0, 1, 2));

    assert!(ids.complete(second));
    assert_eq!(ids.allocate().expect("reused call id"), 1);
}

#[test]
fn test_in_flight_call_ids_reject_duplicate_registration() {
    let mut ids = InFlightCallIds::new();
    ids.register(42).expect("first registration succeeds");

    assert_eq!(ids.register(42), Err(RpcError::DuplicateCallId(42)));
}
