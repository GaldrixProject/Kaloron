// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::consts::{
    HEADER_PROTOCOL, HEADER_SERVICE_HASH, HEADER_SERVICE_ID, HEADER_SERVICE_VERSION,
    TEXTUAL_PROTOCOL,
};
use super::super::facade::Facade;
use super::super::message::{HxClientBoundHeader, HxServerBoundHeader};
use super::super::reader::{FrameReader, InboundFrame};
use super::super::utils::HeaderPair;
use super::super::writer::{FrameWriter, FrameWriterBody};
use crate::{RpcError, ServerChannel, ServiceDispatch};
use anyhow::{Context, anyhow, bail};
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{HeaderMap, Method, Request, Response, StatusCode};
use kaloron::{TypeShape, Version};
use kaloron_codec::TextualCodec;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Mutex;
use tokio::task::JoinSet;
use crate::protocol::hx::scope::ScopeRef;

struct HxServerSessionShared {
    version: Version,
    r_core: HxServerResponseCore,
    writer: FrameWriter<TextualCodec, HxClientBoundHeader>,
}

impl HxServerSessionShared {
    fn write_rpc_error(&self, e: RpcError, call_id: u32) -> anyhow::Result<()> {
        self.writer.frame(
            self.version,
            HxClientBoundHeader::ResponseFailure { call_id },
            e,
        )
    }
}

enum HxServerReadStage {
    Frame(InboundFrame<TextualCodec, HxServerBoundHeader>),
    Fault,
    Done,
}

pub(super) struct HxServerChannel {
    call_id: u32,
    frame: HxServerReadStage,
    share: NonNull<HxServerSessionShared>,
}

unsafe impl Send for HxServerChannel {}

unsafe impl Sync for HxServerChannel {}

impl ServerChannel for HxServerChannel {
    async fn read<T: TypeShape>(&mut self) -> anyhow::Result<Option<T>> {
        match &mut self.frame {
            HxServerReadStage::Frame(f) => match f.read_data::<T>(Version::zero()) {
                Ok(v) => {
                    self.frame = HxServerReadStage::Done;
                    Ok(Some(v))
                }
                Err(_) => {
                    self.frame = HxServerReadStage::Fault;
                    Ok(None)
                }
            },
            HxServerReadStage::Fault => {
                bail!("internal state corruption: read on failed frame")
            }
            HxServerReadStage::Done => {
                bail!("internal state corruption: read on consumed frame")
            }
        }
    }

    async fn write<T: TypeShape>(self, m: T) -> anyhow::Result<()> {
        let share = unsafe { self.share.as_ref() };
        let id_set = share.r_core.complete(self.call_id);
        if id_set {
            share.write_rpc_error(RpcError::DuplicateCallId(self.call_id), self.call_id)
        } else {
            share.writer.frame(
                share.version,
                HxClientBoundHeader::ResponseSuccess {
                    call_id: self.call_id,
                },
                m,
            )
        }
    }

    async fn read_fault(&mut self) -> anyhow::Result<()> {
        match &mut self.frame {
            HxServerReadStage::Frame(_) | HxServerReadStage::Fault => {
                self.frame = HxServerReadStage::Done;
                Ok(())
            }
            HxServerReadStage::Done => {
                bail!("internal state corruption: read_fault on consumed frame")
            }
        }
    }

    async fn write_fault(self, mut e: RpcError) -> anyhow::Result<()> {
        let share = unsafe { self.share.as_ref() };
        let id_set = share.r_core.complete(self.call_id);
        if id_set {
            e = RpcError::DuplicateCallId(self.call_id)
        }
        share.write_rpc_error(e, self.call_id)
    }
}

impl HxServerChannel {}

struct HxServerResponseCore {
    in_flight: Mutex<HashMap<u32, bool>>,
}

impl HxServerResponseCore {
    fn new() -> Self {
        Self {
            in_flight: Mutex::new(HashMap::new()),
        }
    }

    /// Try register an in flight id.
    /// If the id is already registered, set its state to poisoned (true).
    /// Otherwise, register it and set its state to normal (false).
    /// Returns the state boolean.
    fn register(&self, id: u32) -> bool {
        let mut map = self.in_flight.lock().unwrap();
        *map.entry(id).and_modify(|v| *v = true).or_insert(false)
    }

    fn complete(&self, id: u32) -> bool {
        self.in_flight
            .lock()
            .unwrap()
            .remove(&id)
            .expect("core map state corrupted")
    }
}

trait HxServerDynDispatch: Send {
    fn dispatch_to(&self, pool: &mut JoinSet<bool>, method: u32, chan: HxServerChannel);
}

struct HxServerDynDispatchInstance<T: Send + Sync, TB: ServiceDispatch<T, HxServerChannel>> {
    value: T,
    _phantom: PhantomData<fn() -> TB>,
}

impl<T: Send + Sync, TB: ServiceDispatch<T, HxServerChannel>> HxServerDynDispatch
    for HxServerDynDispatchInstance<T, TB>
{
    fn dispatch_to(&self, pool: &mut JoinSet<bool>, method: u32, chan: HxServerChannel) {
        let opaque = &self.value as *const T as usize;
        pool.spawn(
            async move { TB::dispatch(unsafe { &*(opaque as *const T) }, method, chan).await },
        );
    }
}

async fn serve(
    dispatch: Box<dyn HxServerDynDispatch>,
    version: Version,
    reader: FrameReader<TextualCodec, HxServerBoundHeader>,
    writer: FrameWriter<TextualCodec, HxClientBoundHeader>,
) -> anyhow::Result<()> {
    // lifted good state to tell if this is a graceful shutdown
    let mut good = true;
    let mut tasks = JoinSet::<bool>::new();
    let share = HxServerSessionShared {
        version,
        r_core: HxServerResponseCore::new(),
        writer,
    };
    let mut reader = reader;
    loop {
        let frame = reader.read_frame().await?;
        let frame = match frame {
            None => bail!("unexpected end of frames without complete"),
            Some(f) => f,
        };
        match frame.header() {
            HxServerBoundHeader::Request { call_id, method_id } => {
                let call_id = *call_id;
                let method_id = *method_id;
                if !share.r_core.register(call_id) {
                    let chan = HxServerChannel {
                        call_id,
                        frame: HxServerReadStage::Frame(frame),
                        share: NonNull::from_ref(&share),
                    };
                    dispatch.dispatch_to(&mut tasks, method_id, chan);
                } else {
                    share
                        .write_rpc_error(RpcError::DuplicateCallId(call_id), call_id)
                        .expect("infallible");
                }
            }
            HxServerBoundHeader::Complete {} => {
                // graceful shutdown signal.
                // no more frame should be read further
                break;
            }
        }
        // join any ready state in the join set
        loop {
            match tasks.try_join_next() {
                None => break,
                Some(Ok(false)) => {
                    good = false;
                }
                _ => continue,
            }
        }
        if !good {
            break;
        }
    }
    loop {
        match tasks.join_next().await {
            None => break,
            Some(Ok(false)) => {
                good = false;
            }
            _ => continue,
        }
    }
    if good {
        share
            .writer
            .frame(share.version, HxClientBoundHeader::Complete {}, ())
            .expect("infallible")
    } else {
        share
            .writer
            .frame(share.version, HxClientBoundHeader::ChannelError {}, ())
            .expect("infallible")
    }
    Ok(())
}

pub(super) struct UnitJoinSet {
    i: JoinSet<()>,
}

impl UnitJoinSet {
    pub fn new() -> Self {
        Self { i: JoinSet::new() }
    }

    pub fn spawn<F>(&mut self, task: F)
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        self.i.spawn(task);
        loop {
            if self.i.try_join_next().is_none() {
                break;
            }
        }
    }

    pub async fn join(mut self) {
        loop {
            if self.i.join_next().await.is_none() {
                break;
            }
        }
    }
}

pub(super) struct HxRegisteredService {
    is_available_at: fn(Version) -> bool,
    effective_hash: fn(Version) -> [u8; 32],
    runner: Box<dyn Fn() -> Box<dyn HxServerDynDispatch> + Send + Sync>,
}

impl HxRegisteredService {
    pub fn capture<T: Send + Sync + 'static, TB: ServiceDispatch<T, HxServerChannel>>(
        f: impl Fn() -> T + Send + Sync + 'static,
    ) -> HxRegisteredService {
        Self {
            is_available_at: |version| TB::SERVICE_SCHEMA.is_available_at(version),
            effective_hash: |version| TB::effective_schema_hash(version),
            runner: Box::new(move || {
                Box::new(HxServerDynDispatchInstance::<T, TB> {
                    value: f(),
                    _phantom: Default::default(),
                })
            }),
        }
    }
}

pub(super) struct HxServerConsts {
    response_headers: Vec<HeaderPair>,
    services: HashMap<String, HxRegisteredService>,
    frame_limit: usize,
    reader_facade: Facade,
    writer_facade: Facade,
    endpoint: String,
}

impl HxServerConsts {
    pub fn new(
        response_headers: Vec<HeaderPair>,
        services: HashMap<String, HxRegisteredService>,
        frame_limit: usize,
        reader_facade: Facade,
        writer_facade: Facade,
        endpoint: String,
    ) -> Self {
        Self {
            response_headers,
            services,
            frame_limit,
            reader_facade,
            writer_facade,
            endpoint,
        }
    }
}

pub(super) struct HxServerService {
    spawn: ScopeRef,
    consts: NonNull<HxServerConsts>,
}

impl HxServerService {
    pub fn new(join: ScopeRef, consts: &HxServerConsts) -> Self {
        Self {
            spawn: join,
            consts: NonNull::from_ref(consts),
        }
    }

    pub async fn call(
        opaque: usize,
        req: Request<Incoming>,
    ) -> Result<Response<FrameWriterBody>, Infallible> {
        match Self::try_handle_request(opaque, req).await {
            Ok(r) => Ok(r),
            Err(_) => Ok(internal_error_response()),
        }
    }

    async fn try_handle_request(
        opaque: usize,
        request: Request<Incoming>,
    ) -> anyhow::Result<Response<FrameWriterBody>> {
        let this = unsafe { &*(opaque as *const HxServerService) };
        let consts = unsafe { this.consts.as_ref() };
        if request.method() != Method::POST || request.uri().path() != consts.endpoint.as_str() {
            return Ok(not_found_response());
        }

        let headers = request.headers();
        if required_header(headers, HEADER_PROTOCOL)? != TEXTUAL_PROTOCOL {
            return Ok(not_found_response());
        }

        let Some(service) = consts
            .services
            .get(required_header(headers, HEADER_SERVICE_ID)?)
        else {
            return Ok(not_found_response());
        };

        let version = parse_version(required_header(headers, HEADER_SERVICE_VERSION)?)?;
        let hash = parse_hash(required_header(headers, HEADER_SERVICE_HASH)?)?;
        if !(service.is_available_at)(version) || (service.effective_hash)(version) != hash {
            return Ok(not_found_response());
        }

        let dispatch = (service.runner)();

        let writer = FrameWriter::<TextualCodec, HxClientBoundHeader>::new(
            Version::new(0, 1, 0, 0),
            consts.writer_facade.clone(),
        );

        let reader = FrameReader::<TextualCodec, HxServerBoundHeader>::new(
            Version::new(0, 1, 0, 0),
            request.into_body(),
            consts.reader_facade.clone(),
            consts.frame_limit,
        );

        let mut builder = Response::builder().status(StatusCode::OK);
        for header in consts.response_headers.iter() {
            builder = builder.header(&header.name, &header.value);
        }
        let response = builder
            .body(writer.body())
            .context("failed to build h1 success response")?;

        this.spawn.spawn(async move {
            _ = serve(dispatch, version, reader, writer).await;
        });
        Ok(response)
    }
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> anyhow::Result<&'a str> {
    headers
        .get(name)
        .ok_or_else(|| anyhow!("missing required header '{name}'"))?
        .to_str()
        .with_context(|| format!("header '{name}' contains non-UTF-8 value"))
}

fn parse_version(text: &str) -> anyhow::Result<Version> {
    let parts: Vec<&str> = text.split('.').collect();
    if !(3..=4).contains(&parts.len()) {
        bail!(
            "invalid version format '{text}': expected MAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH.BUILD"
        );
    }

    let parse = |value: &str| -> anyhow::Result<u16> {
        value
            .parse::<u16>()
            .with_context(|| format!("invalid version component '{value}'"))
    };

    Ok(Version::new(
        parse(parts[0])?,
        parse(parts[1])?,
        parse(parts[2])?,
        if parts.len() == 4 {
            parse(parts[3])?
        } else {
            0
        },
    ))
}

fn parse_hash(text: &str) -> anyhow::Result<[u8; 32]> {
    if text.len() != 64 {
        bail!("invalid schema hash length: expected 64 hex characters");
    }

    let mut hash = [0u8; 32];
    for (index, chunk) in text.as_bytes().chunks_exact(2).enumerate() {
        let hi = decode_hex_nibble(chunk[0])
            .ok_or_else(|| anyhow!("invalid schema hash hex at byte pair {index}"))?;
        let lo = decode_hex_nibble(chunk[1])
            .ok_or_else(|| anyhow!("invalid schema hash hex at byte pair {index}"))?;
        hash[index] = (hi << 4) | lo;
    }
    Ok(hash)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn not_found_response() -> Response<FrameWriterBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(FrameWriterBody::empty())
        .expect("static 404 response must build")
}

fn internal_error_response() -> Response<FrameWriterBody> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(FrameWriterBody::empty())
        .expect("static 500 response must build")
}

/// exists to avoid boxing
pub(super) struct HxServiceFn<F> {
    s: HxServerService,
    f: F,
}

unsafe impl<F> Send for HxServiceFn<F> where F: Send {}

unsafe impl<F> Sync for HxServiceFn<F> where F: Sync {}

impl<F, Ret> Service<Request<Incoming>> for HxServiceFn<F>
where
    F: Fn(usize, Request<Incoming>) -> Ret,
    Ret: Future<Output = Result<Response<FrameWriterBody>, Infallible>>,
{
    type Response = Response<FrameWriterBody>;
    type Error = Infallible;
    type Future = Ret;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        (self.f)(NonNull::from_ref(&self.s).as_ptr() as usize, req)
    }
}

/// exists to avoid boxing
pub(super) fn hx_service_fn<F, S>(s: HxServerService, f: F) -> HxServiceFn<F>
where
    F: Fn(usize, Request<Incoming>) -> S,
    S: Future,
{
    HxServiceFn { s, f }
}
