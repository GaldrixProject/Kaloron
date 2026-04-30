// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::facade::Facade;
use super::ring::{Frame, FrameRead, RingSpread};
use anyhow::bail;
use bytes::{Buf, Bytes};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use kaloron_codec::ReadExt;
use std::cmp::min;
use std::io::Read;
use std::ptr::NonNull;
use std::sync::Arc;

pub(super) struct InboundFrameInner {
    frame: NonNull<Frame>,
    parent: NonNull<FrameReaderInner>,
    length: usize,
    received: usize,
    offset_item: u32,
    offset_byte: u32,
}

// SAFETY:
// `InboundFrameInner` is moved only as an owned value. Its raw pointers refer to
// stable allocations managed by the reader/ring layer, and routed frames are
// consumed by exactly one task after the reader has fully buffered the frame.
unsafe impl Send for InboundFrameInner {}

impl InboundFrameInner {
    pub(super) fn new(
        frame: NonNull<Frame>,
        parent: &FrameReaderInner,
        length: u32,
        received: usize,
    ) -> Option<Self> {
        Some(Self {
            frame,
            parent: NonNull::from_ref(parent),
            length: length as usize,
            received,
            offset_item: 0,
            offset_byte: 0,
        })
    }
}

impl Read for InboundFrameInner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // SAFETY: InboundFrameInner should never outlive FrameReaderInner.
        match unsafe { self.frame.as_ref() }.read(self.offset_item, self.offset_byte, buf) {
            FrameRead::Full => {
                self.offset_byte += buf.len() as u32;
                Ok(buf.len())
            }
            FrameRead::Trim(l) => {
                self.offset_byte = 0;
                self.offset_item += 1;
                Ok(l as usize)
            }
            FrameRead::EoF => Ok(0),
        }
    }
}

impl ReadExt for InboundFrameInner {
    fn facade<T>(&self) -> anyhow::Result<Arc<T>> {
        // SAFETY: InboundFrameInner should never outlive FrameReaderInner.
        unsafe { self.parent.as_ref() }.facade.facade::<T>()
    }
}

impl Drop for InboundFrameInner {
    fn drop(&mut self) {
        // SAFETY: InboundFrameInner should never outlive FrameReaderInner.
        unsafe { self.parent.as_ref() }.ring.free_frame(self.frame)
    }
}

pub(super) struct FrameReaderInner {
    facade: Facade,
    stream: Incoming,
    ring: RingSpread,
    residue: Option<Bytes>,
    bound: usize,
    #[allow(dead_code)]
    limit: usize, // reserved: frame size enforcement not yet implemented
}

// SAFETY:
// `FrameReaderInner` is driven by a single async state machine at a time.
// Moving the reader future between executor threads transfers ownership rather
// than aliasing access to the raw-pointer-backed ring state.
unsafe impl Send for FrameReaderInner {}

impl FrameReaderInner {
    pub(super) fn new(facade: Facade, stream: Incoming, bound: usize, limit: usize) -> Self {
        Self {
            facade,
            stream,
            ring: RingSpread::new(),
            residue: None,
            bound,
            limit,
        }
    }

    async fn advance_chunk(&mut self) -> anyhow::Result<Option<Bytes>> {
        loop {
            match self.stream.frame().await {
                None => return Ok(None),
                Some(c) => match c?.into_data() {
                    Ok(c) => {
                        if !c.is_empty() {
                            return Ok(Some(c));
                        }
                    }
                    Err(_) => bail!("expecting data segment"),
                },
            }
        }
    }

    async fn advance_chunk_must(&mut self) -> anyhow::Result<Bytes> {
        match self.advance_chunk().await? {
            None => bail!("unexpected end of stream"),
            Some(c) => Ok(c),
        }
    }

    async fn consume_length(&mut self) -> anyhow::Result<Option<u32>> {
        let mut buf = [0u8; 4];
        let mut expect = 4usize;
        let mut b = match self.residue.take() {
            Some(b) => b,
            None => match self.advance_chunk().await? {
                Some(b) => b,
                None => return Ok(None),
            },
        };
        loop {
            let len = b.len();
            if len >= expect {
                b.split_to(expect).copy_to_slice(&mut buf[(4 - expect)..]);
                if !b.is_empty() {
                    self.residue = Some(b)
                };
                return Ok(Some(u32::from_le_bytes(buf)));
            } else {
                b.copy_to_slice(&mut buf[(4 - expect)..]);
                expect -= len;
                b = self.advance_chunk_must().await?;
            }
        }
    }

    pub(super) async fn read_frame_start(&mut self) -> anyhow::Result<Option<InboundFrameInner>> {
        match self.consume_length().await? {
            None => Ok(None),
            Some(size) => {
                let mut buffered = 0;
                let mut pending = Vec::new();
                let mut b = match self.residue.take() {
                    Some(b) => b,
                    None => self.advance_chunk_must().await?,
                };
                loop {
                    let len = b.len();
                    let buffered_next = buffered + len;
                    if buffered_next < min(self.bound, size as usize) {
                        pending.push(b);
                        buffered = buffered_next;
                        b = self.advance_chunk_must().await?;
                    } else if buffered_next <= size as usize {
                        buffered = buffered_next;
                        break;
                    } else {
                        let trim = buffered_next - size as usize;
                        self.residue = Some(b.split_off(b.len() - trim));
                        buffered = size as usize;
                        break;
                    }
                }

                let frame = self.ring.push_frame();
                for bytes in pending {
                    self.ring.push_bytes(bytes);
                }
                self.ring.push_bytes(b);
                Ok(InboundFrameInner::new(frame, self, size, buffered))
            }
        }
    }

    pub(super) async fn finish_frame(
        &mut self,
        frame: &mut InboundFrameInner,
    ) -> anyhow::Result<()> {
        let mut buffered = frame.received;
        if buffered < frame.length {
            loop {
                let mut b = self.advance_chunk_must().await?;
                let len = b.len();
                let buffered_next = buffered + len;
                if buffered_next <= frame.length {
                    buffered = buffered_next;
                    self.ring.push_bytes(b);
                } else {
                    let trim = buffered_next - frame.length;
                    self.residue = Some(b.split_off(b.len() - trim));
                    self.ring.push_bytes(b);
                    break;
                }
            }
            frame.received = frame.length;
        }
        self.ring.seal_frame();
        Ok(())
    }
}
