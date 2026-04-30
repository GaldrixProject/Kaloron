// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::consts::DEFAULT_FRAME_LIMIT;
use super::super::facade::Facade;
use super::super::scope::Scope;
use super::super::traits::HyperListener;
use super::super::utils::HeaderPair;
use super::hx::{
    hx_service_fn, HxRegisteredService, HxServerChannel, HxServerConsts, HxServerService,
    UnitJoinSet,
};
use crate::{
    RpcError, ServerChannel, ServerConfig, ServerTransport, ServiceDispatch, ServiceFactory,
};
use anyhow::{anyhow, bail, Context};
use hyper::http::Uri;
use hyper::server::conn::http1::Builder;
use kaloron::TypeShape;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;
use tokio::task::JoinHandle;

pub struct H1ServerConfig<L: HyperListener> {
    listener: Option<L>,
    endpoint: String,
    extra_headers: Vec<HeaderPair>,
    frame_limit: usize,
    reader_facade: Facade,
    writer_facade: Facade,
    services: HashMap<String, HxRegisteredService>,
}

impl<L: HyperListener> H1ServerConfig<L> {
    pub fn new(listener: L, endpoint: impl Into<String>) -> Self {
        Self {
            listener: Some(listener),
            endpoint: endpoint.into(),
            extra_headers: Vec::new(),
            frame_limit: DEFAULT_FRAME_LIMIT,
            reader_facade: Facade::empty(),
            writer_facade: Facade::empty(),
            services: HashMap::new(),
        }
    }

    pub fn with_extra_header(
        mut self,
        name: hyper::header::HeaderName,
        value: hyper::header::HeaderValue,
    ) -> Self {
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

impl<L: HyperListener> ServerConfig<H1ServerTransport<L>> for H1ServerConfig<L> {
    fn serve<T: Send + Sync + 'static, TB: ServiceDispatch<T, H1ServerChannel>>(
        mut self,
        factory: impl Fn() -> T + Send + Sync + 'static,
    ) -> Self {
        self.services.insert(
            TB::SERVICE_SCHEMA.name.to_string().clone(),
            HxRegisteredService::capture::<T, H1SessionBridge<TB>>(factory),
        );
        self
    }
}

struct H1ServerTransportInner<L: HyperListener> {
    listener: L,
    consts: HxServerConsts,
    _marker: PhantomData<fn() -> L>,
}

impl<L: HyperListener> H1ServerTransportInner<L> {
    async fn run(&self) -> anyhow::Result<()> {
        let mut join = UnitJoinSet::new();
        let result = self.run_accept(&mut join).await;
        join.join().await;
        result
    }

    async fn run_accept(&self, join: &mut UnitJoinSet) -> anyhow::Result<()> {
        let opaque = NonNull::from_ref(self).as_ptr() as usize;
        loop {
            match self.listener.accept().await? {
                None => break,
                Some(stream) => {
                    join.spawn(async move {
                        let _ = Self::run_connection(opaque, stream).await;
                        // TODO: add error reporting mechanism
                    });
                }
            }
        }
        Ok(())
    }

    async fn run_connection(this: usize, io: L::Io) -> anyhow::Result<()> {
        let this = unsafe { &*(this as *const Self) };
        let join = Box::pin(Scope::new());
        let serve = hx_service_fn(
            HxServerService::new(join.as_ref().refer(), &this.consts),
            HxServerService::call,
        );
        let result = Builder::new().serve_connection(io, serve).await;
        join.as_ref().join().await;
        result.context("h1 server connection failed")?;
        Ok(())
    }

    async fn complete(self) {
        self.listener.complete().await;
    }
}

pub struct H1ServerTransport<L: HyperListener> {
    inner: Option<Box<H1ServerTransportInner<L>>>,
    accept_task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl<L: HyperListener> ServerTransport for H1ServerTransport<L> {
    type Config = H1ServerConfig<L>;
    type Channel = H1ServerChannel;

    async fn configure(mut config: Self::Config) -> anyhow::Result<Self> {
        let endpoint: Uri = config
            .endpoint
            .parse()
            .with_context(|| format!("invalid h1 endpoint '{}'", config.endpoint))?;

        if endpoint.scheme().is_some() || endpoint.authority().is_some() {
            bail!("h1 endpoint must be an origin-form path, got '{endpoint}'");
        }
        if !endpoint.path().starts_with('/') {
            bail!("h1 endpoint must start with '/'");
        }
        if config.frame_limit == 0 {
            bail!("h1 frame limit must be greater than zero");
        }

        let listener = config
            .listener
            .take()
            .ok_or_else(|| anyhow!("h1 server config listener already consumed"))?;
        let endpoint = endpoint.to_string();

        let inner = Box::new(H1ServerTransportInner {
            listener,
            consts: HxServerConsts::new(
                config.extra_headers,
                config.services,
                config.frame_limit,
                config.reader_facade,
                config.writer_facade,
                endpoint,
            ),
            _marker: PhantomData,
        });

        let opaque = inner.as_ref() as *const H1ServerTransportInner<L> as usize;
        let accept_task = tokio::spawn(async move {
            let inner = unsafe { &*(opaque as *const H1ServerTransportInner<L>) };
            inner.run().await
        });

        Ok(Self {
            inner: Some(inner),
            accept_task: Some(accept_task),
        })
    }

    async fn complete(mut self) -> anyhow::Result<()> {
        self.inner.take().unwrap().complete().await;
        let result = self.accept_task.take().unwrap().await;
        mem::forget(self);
        match result {
            Ok(result) => result,
            Err(_) => Err(anyhow!("h1 server accept task failed")),
        }
    }
}

impl<L: HyperListener> Drop for H1ServerTransport<L> {
    fn drop(&mut self) {
        let inner = self.inner.take().unwrap();
        let accept_task = self.accept_task.take().unwrap();
        tokio::spawn(async move {
            inner.complete().await;
            _ = accept_task.await;
        });
    }
}

pub struct H1ServerChannel(HxServerChannel);

impl ServerChannel for H1ServerChannel {
    async fn read<T: TypeShape>(&mut self) -> anyhow::Result<Option<T>> {
        self.0.read().await
    }

    async fn write<T: TypeShape>(self, m: T) -> anyhow::Result<()> {
        self.0.write(m).await
    }

    async fn read_fault(&mut self) -> anyhow::Result<()> {
        self.0.read_fault().await
    }

    async fn write_fault(self, e: RpcError) -> anyhow::Result<()> {
        self.0.write_fault(e).await
    }
}

struct H1SessionBridge<TB>(PhantomData<TB>);

impl<TB: ServiceFactory> ServiceFactory for H1SessionBridge<TB> {
    const SERVICE_SCHEMA: crate::ServiceSchema<'static> = TB::SERVICE_SCHEMA;
}

impl<T, TB> ServiceDispatch<T, HxServerChannel> for H1SessionBridge<TB>
where
    T: Send + Sync + 'static,
    TB: ServiceDispatch<T, H1ServerChannel>,
{
    async fn dispatch(handler: &T, method_id: u32, channel: HxServerChannel) -> bool {
        TB::dispatch(handler, method_id, H1ServerChannel(channel)).await
    }
}
