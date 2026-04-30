// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::{ClientChannel, ServerChannel, ServiceDispatch};
use kaloron::Version;

#[allow(async_fn_in_trait)]
pub trait ClientTransport: Sized {
    /// Config data for this transfer
    type Config;

    /// Server side channel type for this transport
    type Channel: ClientChannel;

    /// Configure a new instance of the transport using the given config.
    /// This method should be always fully threadsafe
    async fn configure(config: Self::Config) -> anyhow::Result<Self>;

    /// Open a channel that connects to the server with the given id, version and schema hash.
    /// The server is expected to verify the handshake and accept the connection if the handshake is valid.
    /// If the handshake process fails, the action returns Err with appropriate error message.
    /// This method should be always fully threadsafe
    async fn connect(
        &mut self,
        id: String,
        version: Version,
        hash: &[u8; 32],
    ) -> anyhow::Result<Self::Channel>;

    /// Complete the current transport instance.
    /// Upon this action being called, the internal state of the transport is voided.
    /// Any further call to this instance should fail with instance voided error.
    /// this action returns immediately after the successful shutdown of all live channels.
    /// This method should be always fully threadsafe
    async fn complete(self) -> anyhow::Result<()>;
}

pub trait ServerConfig<TT: ServerTransport>: Sized {
    fn serve<T: Send + Sync + 'static, TB: ServiceDispatch<T, TT::Channel>>(
        self,
        factory: impl Fn() -> T + Send + Sync + 'static,
    ) -> Self;
}

#[allow(async_fn_in_trait)]
pub trait ServerTransport: Sized {
    /// Config data for this transfer
    type Config: ServerConfig<Self>;

    /// Server side channel type for this transport
    type Channel: ServerChannel;

    /// Configure a new instance of the transport using the given config.
    /// A background task is immediately started to accept and serve incoming connections.
    /// Limits, throughput control, and error reporting surfaces should be all provided in config.
    /// Upon successful configuration of listener, task and all data structures,
    /// this function returns the instance used to graceful service shutdown.
    /// Otherwise, an error is reported and no global state should be affected.
    /// This method should be always fully threadsafe
    async fn configure(config: Self::Config) -> anyhow::Result<Self>;

    /// Complete the current transport instance.
    /// Upon this action being called, the internal state of the transport is voided.
    /// Any further call to this instance should fail with instance voided error.
    /// If there is any accept action waiting for connection, it is completed immediately with None.
    /// A server closing message is sent to all open channels,
    /// and the client must start the graceful shutdown process immediately.
    /// After a configured timeout if shutdown is not requested,
    /// a server disconnect message should be sent to all clients,
    /// and the shutdown process starts immediately as if the client has requested shutdown.
    /// this action returns immediately after the successful shutdown of all live channels.
    /// This action is designed to be used in the server instance listener loop async state machine,
    /// prevention of tearing and desync data when transferred between threads is needed.
    async fn complete(self) -> anyhow::Result<()>;
}
