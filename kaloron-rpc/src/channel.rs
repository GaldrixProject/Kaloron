// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::{RpcError, RpcResult, ServiceSchema};
use kaloron::{TypeShape, Version};
use std::future::Future;

// ---------------------------------------------------------------------------
// ServiceFactory
// ---------------------------------------------------------------------------

/// Marker trait for service factory types.
///
/// Each `#[kaloron_rpc]` expansion generates a unique factory struct and
/// implements this trait for it, supplying the static [`ServiceSchema`].
pub trait ServiceFactory: Send + Sync + 'static {
    /// The static schema that describes this service.
    const SERVICE_SCHEMA: ServiceSchema<'static>;

    /// Returns the exact schema hash of the effective service surface at
    /// `version`.
    fn effective_schema_hash(version: Version) -> [u8; 32] {
        Self::SERVICE_SCHEMA.effective_schema_hash(version)
    }
}

pub trait ClientChannel {
    /// Call the remote service with a specified method_id and packaged parameters.
    /// The channel automatically manages the call id.
    /// If an id allocation is not possible, which is highly unlikely,
    /// the request should fail with an id exhaustion error.
    /// The channel should not check the validity of the types for method_id.
    /// If the received response message is a fault, the fault is returned in the result.
    /// This operation is expected to be thread safe.
    /// Any desync between thread or async state machine is not allowed,
    /// any mutation caused by this action must be immediately available to all reference holders.
    fn call<TP: TypeShape, TR: TypeShape>(
        &self,
        method_id: u32,
        parameters: TP,
    ) -> impl Future<Output = anyhow::Result<RpcResult<TR>>> + Send + '_;

    /// Request the channel to do a graceful shutdown by sending a shutdown message.
    /// The internal state of the channel is voided immediately after the initiation of this call.
    /// All request to channel after the initiation of this call fails.
    /// If this operation succeeds, the future of this async call completes when the server side
    /// replies with its shutdown message and closes the channel.
    /// Returns unit if the shutdown operation is successful.
    /// Returns Err if the channel implementation breaks down.
    /// This operation is expected to be thread safe.
    /// Any desync between thread or async state machine is not allowed,
    /// any mutation caused by this action must be immediately available to all reference holders.
    fn complete(self) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// Represents the per-service channel data operation surface which is used by the dispatcher.
/// This is different from the ClientChannel which is per service connection.
/// This type is to be consumed by the service dispatcher method in a serial fashion.
/// Any synchronization needed with the service connection should be done internally.
pub trait ServerChannel: Send + Sync {
    /// Fully receive & decode the request message.
    /// The type need to match what is contained in the message.
    /// The channel does not validate if the message is valid for the method.
    /// This function can return synchronously if frame is already fully received.
    /// This function may always complete synchronously on stream based connection.
    /// Returns the message if the type matches and message fully consumed and decoded.
    /// Returns None if the type does not match. read_fault must be called next.
    /// Returns Err if the channel implementation breaks down.
    fn read<T: TypeShape>(&mut self)
    -> impl Future<Output = anyhow::Result<Option<T>>> + Send + '_;

    /// TODO: encoding is failable for custom types. The current write signature
    ///       returns `anyhow::Result<()>` but some encoding failures (e.g. I/O
    ///       errors during serialization) may need special handling. Consider
    ///       whether this should surface a protocol-level fault or remain a
    ///       transport error.
    /// Fully encode & send the response message.
    /// The channel does not validate if the message is valid for the method.
    /// The operation does not guarantee the message is delivered to the client.
    /// This function can return synchronously if transport employs buffer for sending.
    /// This function may always complete synchronously on transport with unbounded send buffer.
    /// Returns unit if the send operation is successful.
    /// Returns Err if the channel implementation breaks down.
    fn write<T: TypeShape>(self, m: T) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Unconditionally fully consume the message without decoding the data.
    /// Used when a fault occurs and the channel needs to be cleared of the pending message.
    /// This function can return synchronously if frame is already fully received.
    /// This function may always complete synchronously on stream based connection.
    /// Returns the message with unit data when operation succeeds.
    /// Returns Err if the channel implementation breaks down.
    fn read_fault(&mut self) -> impl Future<Output = anyhow::Result<()>> + Send + '_;

    /// Fully send the next response message to a request faulted due to invalid request message.
    /// This function can return synchronously if transport employs buffer for sending.
    /// This function may always complete synchronously on transport with unbounded send buffer.
    /// Returns unit if the send operation is successful.
    /// Returns Err if the channel implementation breaks down.
    /// This operation is expected to be thread safe.
    fn write_fault(self, e: RpcError) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait ServiceDispatch<TH, TC: ServerChannel>: ServiceFactory {
    /// Dispatches one call of the service
    fn dispatch<'a>(
        handler: &'a TH,
        method_id: u32,
        channel: TC,
    ) -> impl Future<Output = bool> + Send + 'a;
}
