// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::protocol::HyperListener;
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

pub struct TcpHyperListener {
    listener: TcpListener,
    shutdown: Arc<Notify>,
}

impl TcpHyperListener {
    pub fn new(listener: TcpListener) -> Self {
        Self {
            listener,
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub async fn bind(addr: impl tokio::net::ToSocketAddrs) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self::new(listener))
    }
}

impl HyperListener for TcpHyperListener {
    type Io = TokioIo<TcpStream>;

    fn accept(&self) -> impl Future<Output = anyhow::Result<Option<Self::Io>>> + Send + '_ {
        let shutdown = self.shutdown.clone();
        async move {
            tokio::select! {
                _ = shutdown.notified() => Ok(None),
                result = self.listener.accept() => match result {
                    Ok((stream, _)) => Ok(Some(TokioIo::new(stream))),
                    Err(e) => Err(anyhow::anyhow!("tcp accept failed: {e}")),
                },
            }
        }
    }

    fn complete(&self) -> impl Future<Output = ()> + Send + '_ {
        let shutdown = self.shutdown.clone();
        async move { shutdown.notify_waiters() }
    }
}
