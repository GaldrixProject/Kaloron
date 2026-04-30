// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::TypeShape;

/// Errors that can occur during RPC operations.
#[derive(Debug, Clone, PartialEq, Eq, TypeShape)]
#[kaloron(introduced = "0.1.0")]
pub enum RpcError {
    /// The requested service identifier is not recognized.
    /// Even if the service exists and rejected due to not in version range,
    /// this is still the applicable error code to ensure consistency.
    #[kaloron(id = 0)]
    UnknownService(String),

    /// The requested method ID is not recognized.
    /// Even if the method exists and rejected due to not in version range,
    /// this is still the applicable error code to ensure consistency.
    #[kaloron(id = 1)]
    UnknownMethod(u32),

    /// The requested payload protocol is not supported.
    #[kaloron(id = 2)]
    UnsupportedProtocol {},

    /// The client and server disagree on the effective schema for a version.
    #[kaloron(id = 3)]
    SchemaHashMismatch {
        #[kaloron(id = 0)]
        service: String,
        #[kaloron(id = 1)]
        version: String,
        #[kaloron(id = 2)]
        client_hash: [u8; 32],
        #[kaloron(id = 3)]
        server_hash: [u8; 32],
    },

    /// An incoming call reused an ID that is already in flight.
    /// This means there is a bug in client implementation.
    #[kaloron(id = 4)]
    DuplicateCallId(u32),

    /// No additional call IDs are currently available.
    /// Generally this should not be possible for a single client.
    /// If it happens, either there is a bug in client implementation,
    /// or you need to rethink your service architecture.
    #[kaloron(id = 5)]
    CallIdExhausted,

    /// A wire-decoding failure.
    #[kaloron(id = 6)]
    DecodeError(String),

    /// A wire-encoding failure.
    #[kaloron(id = 7)]
    EncodeError(String),

    /// An error reported by the underlying transport.
    #[kaloron(id = 8)]
    TransportError(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownService(service) => write!(f, "unknown service: {service}"),
            Self::UnknownMethod(id) => write!(f, "unknown method: {id:#x}"),
            Self::UnsupportedProtocol {} => {
                write!(f, "unsupported RPC protocol")
            }
            Self::SchemaHashMismatch {
                service,
                version,
                client_hash,
                server_hash,
            } => {
                write!(
                    f,
                    "schema hash mismatch for service '{service}' at version {version}: client={}, server={}",
                    format_hash(client_hash),
                    format_hash(server_hash),
                )
            }
            Self::DuplicateCallId(call_id) => write!(f, "duplicate call id: {call_id}"),
            Self::CallIdExhausted => f.write_str("no available call ids remain"),
            Self::DecodeError(msg) => write!(f, "decode error: {msg}"),
            Self::EncodeError(msg) => write!(f, "encode error: {msg}"),
            Self::TransportError(msg) => write!(f, "transport error: {msg}"),
        }
    }
}

impl std::error::Error for RpcError {}

/// Outer RPC-layer result alias used by service traits and dispatch calls.
///
/// `RpcResult<T>` represents the framework-level outcome of an RPC operation.
/// The success payload `T` may itself be a `Result<Ok, DomainError>` when the
/// operation needs an application-specific error layer that is distinct from
/// RPC/transport failures.
pub type RpcResult<T> = Result<T, RpcError>;

fn format_hash(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
