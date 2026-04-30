// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::{ClientChannel, ClientTransport, ServiceFactory};
use kaloron::Version;
use std::marker::PhantomData;

/// Transport-level client.
///
/// Wraps a [`ClientTransport`] instance and manages the underlying transport
/// connection. This is the lower-level owner type; the macro-generated
/// `<ServiceName>Client<TT>` alias resolves to [`ServiceClient<TT, Factory>`]
/// and operates in terms of a concrete transport channel.
pub struct Client<TT: ClientTransport> {
    transport: TT,
}

impl<TT: ClientTransport> Client<TT> {
    pub async fn new(config: TT::Config) -> anyhow::Result<Self> {
        Ok(Self {
            transport: TT::configure(config).await?,
        })
    }

    pub async fn connect<Ts: ServiceFactory>(
        &mut self,
        version: Version,
    ) -> anyhow::Result<ServiceClient<TT, Ts>> {
        let channel = self
            .transport
            .connect(
                Ts::SERVICE_SCHEMA.name.to_string(),
                version,
                &Ts::effective_schema_hash(version),
            )
            .await?;
        Ok(ServiceClient::new(channel))
    }

    pub async fn complete(self) -> anyhow::Result<()> {
        self.transport.complete().await
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Generic RPC client for Service.
///
/// Service traits are implemented on `ServiceClient<Td, Ts>` by the macro-generated
/// code, so the client can be used directly as the requested service.
pub struct ServiceClient<TT: ClientTransport, Ts> {
    channel: TT::Channel,
    _service: PhantomData<fn(Ts) -> Ts>,
}

impl<TT: ClientTransport, Ts: ServiceFactory> ServiceClient<TT, Ts> {
    /// Create a new client backed by `channel`.
    fn new(channel: TT::Channel) -> Self {
        Self {
            channel,
            _service: PhantomData,
        }
    }

    /// Borrow the underlying transport channel used for service calls.
    pub fn channel(&self) -> &TT::Channel {
        &self.channel
    }

    /// completes the channel
    pub async fn complete(self) -> anyhow::Result<()> {
        self.channel.complete().await
    }
}
