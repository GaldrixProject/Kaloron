// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::protocol::HyperListener;
use anyhow::Context;
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio_rustls::TlsAcceptor;

type TlsStream = tokio_rustls::server::TlsStream<TcpStream>;

pub struct TlsHyperListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    shutdown: Arc<Notify>,
}

impl TlsHyperListener {
    pub fn new(listener: TcpListener, config: Arc<rustls::ServerConfig>) -> Self {
        Self {
            listener,
            acceptor: TlsAcceptor::from(config),
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub async fn bind(
        addr: impl tokio::net::ToSocketAddrs,
        config: Arc<rustls::ServerConfig>,
    ) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self::new(listener, config))
    }
}

impl HyperListener for TlsHyperListener {
    type Io = TokioIo<TlsStream>;

    fn accept(&self) -> impl Future<Output = anyhow::Result<Option<Self::Io>>> + Send + '_ {
        let shutdown = self.shutdown.clone();
        async move {
            tokio::select! {
                _ = shutdown.notified() => Ok(None),
                result = self.listener.accept() => {
                    let (stream, _) = result.context("tcp accept failed")?;
                    let tls = self.acceptor.accept(stream).await.context("tls handshake failed")?;
                    Ok(Some(TokioIo::new(tls)))
                },
            }
        }
    }

    fn complete(&self) -> impl Future<Output = ()> + Send + '_ {
        let shutdown = self.shutdown.clone();
        async move { shutdown.notify_waiters() }
    }
}
