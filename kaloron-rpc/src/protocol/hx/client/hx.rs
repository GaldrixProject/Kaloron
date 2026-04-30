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
use crate::{ClientChannel, RpcError, RpcResult};
use anyhow::{Context, anyhow, bail};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode, Uri};
use kaloron::{TypeShape, Version};
use kaloron_codec::TextualCodec;
use std::cell::{Cell, UnsafeCell};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, oneshot};
use tokio::task::JoinHandle;

type PendingFrame = InboundFrame<TextualCodec, HxClientBoundHeader>;
type PendingResult = anyhow::Result<PendingFrame>;

struct PendingCalls {
    next_candidate: u32,
    waiters: BTreeMap<u32, oneshot::Sender<PendingResult>>,
}

impl PendingCalls {
    fn new() -> Self {
        Self {
            next_candidate: 0,
            waiters: BTreeMap::new(),
        }
    }

    fn allocate(&mut self, waiter: oneshot::Sender<PendingResult>) -> Result<u32, RpcError> {
        let call_id = self
            .find_first_available_from(self.next_candidate)
            .ok_or(RpcError::CallIdExhausted)?;

        let previous = self.waiters.insert(call_id, waiter);
        debug_assert!(previous.is_none(), "allocated an already-pending call id");
        self.refresh_next_candidate_from(call_id.wrapping_add(1));
        Ok(call_id)
    }

    fn complete(&mut self, call_id: u32) -> Option<oneshot::Sender<PendingResult>> {
        let sender = self.waiters.remove(&call_id);
        if sender.is_some() && call_id < self.next_candidate {
            self.next_candidate = call_id;
        }
        sender
    }

    fn is_empty(&self) -> bool {
        self.waiters.is_empty()
    }

    fn take_all(&mut self) -> BTreeMap<u32, oneshot::Sender<PendingResult>> {
        std::mem::take(&mut self.waiters)
    }

    fn find_first_available_from(&self, start: u32) -> Option<u32> {
        let mut candidate = start;
        loop {
            if !self.waiters.contains_key(&candidate) {
                return Some(candidate);
            }

            candidate = candidate.wrapping_add(1);
            if candidate == start {
                return None;
            }
        }
    }

    fn refresh_next_candidate_from(&mut self, start: u32) {
        self.next_candidate = self.find_first_available_from(start).unwrap_or(start);
    }
}

enum ChannelLifecycle {
    Open,
    Completing,
    Closed,
    Broken,
}

struct HxClientChannelState {
    lifecycle: ChannelLifecycle,
    shutdown_acknowledged: bool,
    pending_calls: PendingCalls,
    terminal_message: Option<String>,
}

impl HxClientChannelState {
    fn new() -> Self {
        Self {
            lifecycle: ChannelLifecycle::Open,
            shutdown_acknowledged: false,
            pending_calls: PendingCalls::new(),
            terminal_message: None,
        }
    }
}

struct HxClientChannelShared {
    reader: UnsafeCell<FrameReader<TextualCodec, HxClientBoundHeader>>,
    reader_taken: Cell<bool>,
    state: Mutex<HxClientChannelState>,
    state_notify: Notify,
}

// SAFETY:
// - `state` remains synchronized by its mutex.
// - `take_reader` yields the only mutable reference to `reader`, guarded by
//   `reader_taken`, and that method is only used by the receiver task startup
//   path.
// - No other code path accesses `reader` directly while that mutable reference
//   is live.
unsafe impl Sync for HxClientChannelShared {}

impl HxClientChannelShared {
    fn new(reader: FrameReader<TextualCodec, HxClientBoundHeader>) -> Self {
        Self {
            reader: UnsafeCell::new(reader),
            reader_taken: Cell::new(false),
            state: Mutex::new(HxClientChannelState::new()),
            state_notify: Notify::new(),
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn take_reader(&self) -> anyhow::Result<&mut FrameReader<TextualCodec, HxClientBoundHeader>> {
        if self.reader_taken.replace(true) {
            return Err(anyhow!("hx client receiver reader was already taken"));
        }

        // SAFETY:
        // - `reader_taken` guarantees this branch runs at most once.
        // - The returned mutable reference is used only by the receiver task.
        // - No other code path accesses `reader` directly.
        Ok(unsafe { &mut *self.reader.get() })
    }

    fn terminal_message(&self) -> Option<String> {
        self.state
            .lock()
            .expect("hx client state mutex poisoned")
            .terminal_message
            .clone()
    }

    fn shutdown_call_error(&self) -> RpcError {
        RpcError::TransportError("channel is shutting down or already closed".to_owned())
    }

    fn route_response(&self, call_id: u32, frame: PendingFrame) -> anyhow::Result<()> {
        let sender = {
            let mut state = self.state.lock().expect("hx client state mutex poisoned");
            state.pending_calls.complete(call_id)
        };

        match sender {
            Some(sender) => {
                let _ = sender.send(Ok(frame));
                Ok(())
            }
            None => {
                let error = anyhow!("received response for unknown call id {call_id}");
                self.break_channel(error.to_string());
                Err(error)
            }
        }
    }

    fn acknowledge_shutdown(&self) {
        let should_notify = {
            let mut state = self.state.lock().expect("hx client state mutex poisoned");
            state.shutdown_acknowledged = true;
            match state.lifecycle {
                ChannelLifecycle::Open => {
                    state.lifecycle = ChannelLifecycle::Completing;
                    true
                }
                ChannelLifecycle::Completing => true,
                ChannelLifecycle::Closed | ChannelLifecycle::Broken => false,
            }
        };

        if should_notify {
            self.state_notify.notify_waiters();
        }
    }

    fn break_channel(&self, message: String) {
        let pending = {
            let mut state = self.state.lock().expect("hx client state mutex poisoned");
            match state.lifecycle {
                ChannelLifecycle::Closed | ChannelLifecycle::Broken => return,
                ChannelLifecycle::Open | ChannelLifecycle::Completing => {
                    state.lifecycle = ChannelLifecycle::Broken;
                    state.terminal_message = Some(message.clone());
                    state.pending_calls.take_all()
                }
            }
        };

        for (_, sender) in pending {
            let _ = sender.send(Err(anyhow!(message.clone())));
        }

        self.state_notify.notify_waiters();
    }

    fn finish_eof(&self) -> anyhow::Result<()> {
        let (graceful, message, pending) = {
            let mut state = self.state.lock().expect("hx client state mutex poisoned");
            if state.shutdown_acknowledged && state.pending_calls.is_empty() {
                state.lifecycle = ChannelLifecycle::Closed;
                (true, None, BTreeMap::new())
            } else {
                let message = if state.shutdown_acknowledged {
                    "channel closed before all pending calls were answered".to_owned()
                } else {
                    "channel closed unexpectedly".to_owned()
                };
                state.lifecycle = ChannelLifecycle::Broken;
                state.terminal_message = Some(message.clone());
                (false, Some(message), state.pending_calls.take_all())
            }
        };

        if let Some(message) = &message {
            for (_, sender) in pending {
                let _ = sender.send(Err(anyhow!(message.clone())));
            }
        }

        self.state_notify.notify_waiters();

        if graceful {
            Ok(())
        } else {
            Err(anyhow!(
                message.expect("broken channel must carry a terminal message")
            ))
        }
    }

    async fn wait_for_terminal_state(&self) -> anyhow::Result<()> {
        loop {
            let notified = self.state_notify.notified();
            match self
                .state
                .lock()
                .expect("hx client state mutex poisoned")
                .lifecycle
            {
                ChannelLifecycle::Closed => return Ok(()),
                ChannelLifecycle::Broken => {
                    let message = self
                        .terminal_message()
                        .unwrap_or_else(|| "channel broke without a diagnostic".to_owned());
                    return Err(anyhow!(message));
                }
                ChannelLifecycle::Open | ChannelLifecycle::Completing => {}
            }
            notified.await;
        }
    }
}

pub(super) struct HxClientChannel {
    version: Version,
    writer: FrameWriter<TextualCodec, HxServerBoundHeader>,
    shared: Arc<HxClientChannelShared>,
    receiver_task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl HxClientChannel {
    pub(in crate::protocol::hx) fn new(
        version: Version,
        reader: FrameReader<TextualCodec, HxClientBoundHeader>,
        writer: FrameWriter<TextualCodec, HxServerBoundHeader>,
    ) -> Self {
        let shared = Arc::new(HxClientChannelShared::new(reader));
        let receiver_shared = Arc::clone(&shared);
        let receiver_task = tokio::spawn(async move { Self::run_receiver(receiver_shared).await });

        Self {
            version,
            writer,
            shared,
            receiver_task: Some(receiver_task),
        }
    }

    async fn run_receiver(shared: Arc<HxClientChannelShared>) -> anyhow::Result<()> {
        let reader = shared.take_reader()?;

        loop {
            let frame = match reader.read_frame().await {
                Ok(Some(frame)) => frame,
                Ok(None) => return shared.finish_eof(),
                Err(error) => {
                    shared.break_channel(format!("failed to read inbound frame: {error}"));
                    return Err(error).context("hx client receiver failed to read next frame");
                }
            };

            match frame.header() {
                HxClientBoundHeader::ResponseSuccess { call_id }
                | HxClientBoundHeader::ResponseFailure { call_id } => {
                    shared.route_response(*call_id, frame)?;
                }
                HxClientBoundHeader::ChannelError {} => {
                    let error = anyhow!("remote channel error");
                    shared.break_channel(error.to_string());
                    return Err(error);
                }
                HxClientBoundHeader::Complete {} => {
                    shared.acknowledge_shutdown();
                }
            }
        }
    }

    async fn join_receiver_if_needed(&mut self) -> anyhow::Result<()> {
        if let Some(receiver_task) = self.receiver_task.take() {
            receiver_task
                .await
                .context("hx client receiver task failed to join")??;
        }
        Ok(())
    }
}

impl ClientChannel for HxClientChannel {
    async fn call<TP: TypeShape, TR: TypeShape>(
        &self,
        method_id: u32,
        parameters: TP,
    ) -> anyhow::Result<RpcResult<TR>> {
        let (call_id, response_waiter) = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("hx client state mutex poisoned");

            match state.lifecycle {
                ChannelLifecycle::Open => {}
                ChannelLifecycle::Completing | ChannelLifecycle::Closed => {
                    return Ok(Err(self.shared.shutdown_call_error()));
                }
                ChannelLifecycle::Broken => {
                    let message = state
                        .terminal_message
                        .clone()
                        .unwrap_or_else(|| "channel broke without a diagnostic".to_owned());
                    return Err(anyhow!(message));
                }
            }

            let (response_sender, response_waiter) = oneshot::channel();
            let call_id = match state.pending_calls.allocate(response_sender) {
                Ok(call_id) => call_id,
                Err(error) => return Ok(Err(error)),
            };

            (call_id, response_waiter)
        };

        if let Err(error) = self.writer.frame(
            self.version,
            HxServerBoundHeader::Request { call_id, method_id },
            parameters,
        ) {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("hx client state mutex poisoned");
            state.pending_calls.complete(call_id);
            return Ok(Err(RpcError::EncodeError(error.to_string())));
        }

        let mut frame = match response_waiter.await {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => return Err(error),
            Err(_) => {
                let message = self.shared.terminal_message().unwrap_or_else(|| {
                    format!("response waiter for call id {call_id} was dropped")
                });
                return Err(anyhow!(message));
            }
        };

        match frame.header() {
            HxClientBoundHeader::ResponseSuccess { .. } => frame
                .read_data::<TR>(self.version)
                .map(Ok)
                .map_err(|error| anyhow!(RpcError::DecodeError(error.to_string()))),
            HxClientBoundHeader::ResponseFailure { .. } => frame
                .read_data::<RpcError>(self.version)
                .map(Err)
                .map_err(|error| anyhow!(RpcError::DecodeError(error.to_string()))),
            HxClientBoundHeader::ChannelError {} => {
                Err(anyhow!("unexpected routed channel error frame"))
            }
            HxClientBoundHeader::Complete {} => {
                Err(anyhow!("unexpected routed shutdown acknowledgement frame"))
            }
        }
    }

    async fn complete(mut self) -> anyhow::Result<()> {
        let should_send_shutdown = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("hx client state mutex poisoned");
            match state.lifecycle {
                ChannelLifecycle::Open => {
                    state.lifecycle = ChannelLifecycle::Completing;
                    true
                }
                ChannelLifecycle::Completing => false,
                ChannelLifecycle::Closed => false,
                ChannelLifecycle::Broken => {
                    let message = state
                        .terminal_message
                        .clone()
                        .unwrap_or_else(|| "channel broke without a diagnostic".to_owned());
                    return Err(anyhow!(message));
                }
            }
        };

        if should_send_shutdown
            && let Err(error) =
                self.writer
                    .frame(self.version, HxServerBoundHeader::Complete {}, ())
        {
            self.shared
                .break_channel(format!("failed to encode graceful shutdown frame: {error}"));
        }

        let wait_result = self.shared.wait_for_terminal_state().await;
        let join_result = self.join_receiver_if_needed().await;

        match (wait_result, join_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Err(wait_error), Err(join_error)) => Err(wait_error.context(join_error.to_string())),
        }
    }
}

impl Drop for HxClientChannel {
    fn drop(&mut self) {
        self.shared
            .break_channel("channel dropped before graceful completion".to_owned());
        if let Some(receiver_task) = self.receiver_task.take() {
            receiver_task.abort();
        }
    }
}
pub(super) trait HxClientRequester {
    fn send(
        &self,
        req: Request<FrameWriterBody>,
    ) -> impl Future<Output = anyhow::Result<Response<Incoming>>>;
}

pub(super) struct HxClientChannelBuilder {
    endpoint: Uri,
    extra_headers: Vec<HeaderPair>,
    frame_limit: usize,
    reader_facade: Facade,
    writer_facade: Facade,
}

impl HxClientChannelBuilder {
    pub fn new(
        endpoint: Uri,
        extra_headers: Vec<HeaderPair>,
        frame_limit: usize,
        reader_facade: Facade,
        writer_facade: Facade,
    ) -> Self {
        Self {
            endpoint,
            extra_headers,
            frame_limit,
            reader_facade,
            writer_facade,
        }
    }

    pub async fn build(
        &self,
        id: String,
        version: Version,
        hash: &[u8; 32],
        send: &impl HxClientRequester,
    ) -> anyhow::Result<HxClientChannel> {
        let mut request_builder = Request::post(self.endpoint.clone())
            .header(HEADER_PROTOCOL, TEXTUAL_PROTOCOL)
            .header(HEADER_SERVICE_ID, id)
            .header(HEADER_SERVICE_VERSION, version.to_string())
            .header(HEADER_SERVICE_HASH, encode_hash(hash));

        for header in &self.extra_headers {
            request_builder = request_builder.header(&header.name, &header.value);
        }

        let writer = FrameWriter::<TextualCodec, HxServerBoundHeader>::new(
            Version::new(0, 1, 0, 0),
            self.writer_facade.clone(),
        );

        let request = request_builder
            .body(writer.body())
            .context("failed to build client request")?;

        let response = send.send(request).await?;

        if response.status() != StatusCode::OK {
            let status = response.status();
            let canonical = status.canonical_reason().unwrap_or("unknown status");
            bail!("negotiation failed with HTTP {status} ({canonical})");
        }

        let reader = FrameReader::<TextualCodec, HxClientBoundHeader>::new(
            Version::new(0, 1, 0, 0),
            response.into_body(),
            self.reader_facade.clone(),
            self.frame_limit,
        );

        Ok(HxClientChannel::new(version, reader, writer))
    }
}

fn encode_hash(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
