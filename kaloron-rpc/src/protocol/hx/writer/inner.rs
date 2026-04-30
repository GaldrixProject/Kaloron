// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::facade::Facade;
use super::ring::{FrameBuilder, RingGather};
use bytes::BytesMut;
use kaloron::Version;
use kaloron_codec::WriteExt;
use std::io::{Error, ErrorKind, Write};
use std::ptr::NonNull;
use std::task::Waker;

const PAYLOAD_CHUNK_SIZE: usize = 1024;
const FRAME_LENGTH_SIZE: usize = size_of::<u32>();

pub(super) struct OutboundFrameInner {
    i: FrameBuilder,
    facade: NonNull<Facade>,
    frame_length_ptr: NonNull<u8>,
    payload_len: u32,
    chunk: BytesMut,
    chunk_len: usize,
}

impl OutboundFrameInner {
    pub(super) fn new(mut i: FrameBuilder, facade: &Facade) -> Self {
        let mut frame_length = BytesMut::zeroed(FRAME_LENGTH_SIZE);
        let frame_length_ptr = NonNull::new(frame_length.as_mut_ptr())
            .expect("zero-length frame header buffer is impossible");
        i.append_frame(frame_length.freeze());

        Self {
            i,
            facade: NonNull::from_ref(facade),
            frame_length_ptr,
            payload_len: 0,
            chunk: Self::allocate_chunk(),
            chunk_len: 0,
        }
    }

    fn allocate_chunk() -> BytesMut {
        BytesMut::zeroed(PAYLOAD_CHUNK_SIZE)
    }

    fn payload_overflow_error() -> Error {
        Error::new(
            ErrorKind::InvalidData,
            "outbound frame payload exceeds u32::MAX octets",
        )
    }

    fn flush_full_chunk(&mut self) {
        debug_assert_eq!(self.chunk_len, PAYLOAD_CHUNK_SIZE);
        let full_chunk = std::mem::replace(&mut self.chunk, Self::allocate_chunk());
        self.i.append_frame(full_chunk.freeze());
        self.chunk_len = 0;
    }

    pub(super) fn finish(mut self) -> anyhow::Result<()> {
        if self.chunk_len != 0 {
            self.chunk.truncate(self.chunk_len);
            self.i.append_frame(self.chunk.freeze());
        }

        let payload_len = self.payload_len.to_le_bytes();
        // SAFETY:
        // - `frame_length_ptr` was taken from the first buffer appended into the detached frame.
        // - That buffer is exactly `FRAME_LENGTH_SIZE` bytes long and remains uniquely owned by the
        //   builder/ring until the frame is polled by the body.
        // - The prefix is written exactly once here before the frame is sealed and exposed.
        unsafe {
            std::ptr::copy_nonoverlapping(
                payload_len.as_ptr(),
                self.frame_length_ptr.as_ptr(),
                FRAME_LENGTH_SIZE,
            );
        }

        self.i.seal_frame();
        Ok(())
    }
}

impl Write for OutboundFrameInner {
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
        let written = buf.len();
        let additional = u32::try_from(written).map_err(|_| Self::payload_overflow_error())?;
        self.payload_len = self
            .payload_len
            .checked_add(additional)
            .ok_or_else(Self::payload_overflow_error)?;

        while !buf.is_empty() {
            let remaining = PAYLOAD_CHUNK_SIZE - self.chunk_len;
            let take = remaining.min(buf.len());
            let chunk_end = self.chunk_len + take;
            self.chunk[self.chunk_len..chunk_end].copy_from_slice(&buf[..take]);
            self.chunk_len = chunk_end;
            buf = &buf[take..];

            if self.chunk_len == PAYLOAD_CHUNK_SIZE {
                self.flush_full_chunk();
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl WriteExt for OutboundFrameInner {
    fn facade<T>(&self) -> anyhow::Result<std::sync::Arc<T>> {
        // SAFETY: OutboundFrameInner never outlives its owning FrameWriter.
        unsafe { self.facade.as_ref() }.facade::<T>()
    }
}

pub(super) struct FrameWriterCore {
    pub(super) v: Version,
    pub(super) i: RingGather,
    pub(super) facade: Facade,
    pub(super) waiter: Option<Waker>,
    pub(super) closed: bool,
    pub(super) body_created: bool,
}

unsafe impl Send for FrameWriterCore {}

impl FrameWriterCore {
    pub(super) fn new(v: Version, facade: Facade) -> Self {
        Self {
            v,
            i: RingGather::new(),
            facade,
            waiter: None,
            closed: false,
            body_created: false,
        }
    }
}
