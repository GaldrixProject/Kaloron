// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::{ServerConfig, ServerTransport, ServiceDispatch};

/// Configured server-side RPC router plus graceful-shutdown handle for the
/// underlying transport.
///
/// The transport itself is configured eagerly during [`ServerBuilder::build`].
/// With the current transport traits the accept / connection lifecycle is owned
/// by the transport implementation, while this type owns the registered service
/// metadata and dispatch table used to verify and route incoming requests.
pub struct Server<TT: ServerTransport> {
    transport: TT,
}

impl<TT: ServerTransport> Server<TT> {
    async fn new(config: TT::Config) -> anyhow::Result<Self> {
        Ok(Self {
            transport: TT::configure(config).await?,
        })
    }
    /// Gracefully stop the underlying transport and all of its live channels.
    pub async fn complete(self) -> anyhow::Result<()> {
        self.transport.complete().await
    }
}

pub struct ServerBuilder<TT: ServerTransport> {
    config: TT::Config,
}

impl<TT: ServerTransport> ServerBuilder<TT> {
    pub fn new(config: TT::Config) -> Self {
        Self { config }
    }

    /// Register a service with the server router.
    ///
    /// `factory` is called once per dispatched request to create a fresh
    /// handler instance. `TB` supplies both the static [`ServiceFactory`]
    /// metadata and the generated [`ServiceDispatch`] implementation.
    pub fn serve<T: Send + Sync + 'static, TB: ServiceDispatch<T, TT::Channel>>(
        self,
        factory: impl Fn() -> T + Send + Sync + 'static,
    ) -> Self {
        Self {
            config: self.config.serve::<T, TB>(factory),
        }
    }

    pub async fn build(self) -> anyhow::Result<Server<TT>> {
        Server::<TT>::new(self.config).await
    }
}
