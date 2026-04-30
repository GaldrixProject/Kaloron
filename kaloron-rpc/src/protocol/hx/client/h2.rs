// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::consts::{
    DEFAULT_FRAME_LIMIT, HEADER_PROTOCOL, HEADER_SERVICE_HASH, HEADER_SERVICE_ID,
    HEADER_SERVICE_VERSION,
};
use super::super::facade::Facade;
use super::super::traits::HyperStream;
use super::super::utils::HeaderPair;
use super::super::writer::FrameWriterBody;
use super::hx::{HxClientChannel, HxClientChannelBuilder, HxClientRequester};
use crate::{ClientChannel, ClientTransport, RpcResult};
use anyhow::{Context, bail};
use hyper::Response;
use hyper::body::Incoming;
use hyper::client::conn::http2::{self, SendRequest};
use hyper::http::Uri;
use hyper::{Request, header};
use hyper_util::rt::TokioExecutor;
use kaloron::{TypeShape, Version};
use std::marker::PhantomData;
use std::sync::Mutex;
use tokio::task::JoinHandle;

struct H2ClientRequester(Mutex<SendRequest<FrameWriterBody>>);

impl HxClientRequester for H2ClientRequester {
    async fn send(&self, req: Request<FrameWriterBody>) -> anyhow::Result<Response<Incoming>> {
        {
            self.0
                .lock()
                .expect("sender mutex poisoned")
                .send_request(req)
        }
        .await
        .context("failed to send h2 client request")
    }
}

pub struct H2ClientConfig<IO> {
    stream: IO,
    endpoint: String,
    extra_headers: Vec<HeaderPair>,
    frame_limit: usize,
    reader_facade: Facade,
    writer_facade: Facade,
}

impl<IO> H2ClientConfig<IO> {
    pub fn new(stream: IO, endpoint: impl Into<String>) -> Self {
        Self {
            stream,
            endpoint: endpoint.into(),
            extra_headers: Vec::new(),
            frame_limit: DEFAULT_FRAME_LIMIT,
            reader_facade: Facade::empty(),
            writer_facade: Facade::empty(),
        }
    }

    pub fn with_extra_header(
        mut self,
        name: header::HeaderName,
        value: header::HeaderValue,
    ) -> Self {
        if matches!(
            name.as_str(),
            HEADER_PROTOCOL | HEADER_SERVICE_ID | HEADER_SERVICE_VERSION | HEADER_SERVICE_HASH
        ) {
            return self;
        }
        self.extra_headers.push(HeaderPair::new(name, value));
        self
    }

    pub fn with_frame_limit(mut self, frame_limit: usize) -> Self {
        self.frame_limit = frame_limit;
        self
    }

    pub fn with_reader_facade(mut self, facade: Facade) -> Self {
        self.reader_facade = facade;
        self
    }

    pub fn with_writer_facade(mut self, facade: Facade) -> Self {
        self.writer_facade = facade;
        self
    }
}

pub struct H2ClientTransport<IO> {
    builder: HxClientChannelBuilder,
    sender_requester: H2ClientRequester,
    connection_task: JoinHandle<anyhow::Result<()>>,
    _phantom: PhantomData<IO>,
}

impl<IO: HyperStream> ClientTransport for H2ClientTransport<IO> {
    type Config = H2ClientConfig<IO>;
    type Channel = H2ClientChannel;

    async fn configure(config: Self::Config) -> anyhow::Result<Self> {
        let endpoint: Uri = config
            .endpoint
            .parse()
            .with_context(|| format!("invalid h2 endpoint '{}'", config.endpoint))?;

        if endpoint.scheme().is_none() {
            bail!("h2 endpoint must have a scheme, got '{endpoint}'");
        }
        if endpoint.authority().is_none() {
            bail!("h2 endpoint must have an authority (host), got '{endpoint}'");
        }
        if !endpoint.path().starts_with('/') {
            bail!("h2 endpoint must have a path starting with '/', got '{endpoint}'");
        }
        if config.frame_limit == 0 {
            bail!("h2 frame limit must be greater than zero");
        }

        let (sender, connection) = http2::Builder::new(TokioExecutor::new())
            .handshake::<IO, FrameWriterBody>(config.stream)
            .await
            .context("failed to start h2 client connection")?;

        let connection_task =
            tokio::spawn(
                async move { connection.await.context("h2 client connection task failed") },
            );

        Ok(Self {
            builder: HxClientChannelBuilder::new(
                endpoint,
                config.extra_headers,
                config.frame_limit,
                config.reader_facade,
                config.writer_facade,
            ),
            sender_requester: H2ClientRequester(Mutex::new(sender)),
            connection_task,
            _phantom: PhantomData,
        })
    }

    async fn connect(
        &mut self,
        id: String,
        version: Version,
        hash: &[u8; 32],
    ) -> anyhow::Result<Self::Channel> {
        self.builder
            .build(id, version, hash, &self.sender_requester)
            .await
            .map(H2ClientChannel)
    }

    async fn complete(self) -> anyhow::Result<()> {
        self.connection_task.abort();
        match self.connection_task.await {
            Ok(Ok(())) | Err(_) => Ok(()),
            Ok(Err(error)) => Err(error),
        }
    }
}

pub struct H2ClientChannel(HxClientChannel);

impl ClientChannel for H2ClientChannel {
    async fn call<TP: TypeShape, TR: TypeShape>(
        &self,
        method_id: u32,
        parameters: TP,
    ) -> anyhow::Result<RpcResult<TR>> {
        self.0.call(method_id, parameters).await
    }

    async fn complete(self) -> anyhow::Result<()> {
        self.0.complete().await
    }
}
