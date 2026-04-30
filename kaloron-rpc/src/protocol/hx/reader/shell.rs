// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::facade::Facade;
use super::inner::{FrameReaderInner, InboundFrameInner};
use kaloron::{TypeShape, Version};
use kaloron_codec::Codec;
use std::marker::PhantomData;

pub struct FrameReader<C: Codec, H: TypeShape> {
    v: Version,
    i: FrameReaderInner,
    _c: PhantomData<fn() -> C>,
    _h: PhantomData<fn() -> H>,
}

impl<C: Codec, H: TypeShape> FrameReader<C, H> {
    pub fn new(
        version: Version,
        stream: hyper::body::Incoming,
        facade: Facade,
        limit: usize,
    ) -> Self {
        let bound =
            C::bound::<H>(version).expect("header bound must be available for split reader");
        Self {
            v: version,
            i: FrameReaderInner::new(facade, stream, bound, limit),
            _c: PhantomData,
            _h: PhantomData,
        }
    }

    pub async fn read_frame(&mut self) -> anyhow::Result<Option<InboundFrame<C, H>>> {
        let mut frame = match self.i.read_frame_start().await? {
            None => return Ok(None),
            Some(f) => f,
        };
        let h = C::decode(self.v, &mut frame)?;
        self.i.finish_frame(&mut frame).await?;
        Ok(Some(InboundFrame::<C, H>::new(h, frame)))
    }
}

unsafe impl<C: Codec, H: TypeShape> Send for FrameReader<C, H> {}

unsafe impl<C: Codec, H: TypeShape> Sync for FrameReader<C, H> {}

pub struct InboundFrame<C: Codec, H: TypeShape> {
    h: H,
    i: InboundFrameInner,
    _c: PhantomData<fn() -> C>,
}

impl<C: Codec, H: TypeShape> InboundFrame<C, H> {
    fn new(h: H, i: InboundFrameInner) -> Self {
        Self {
            h,
            i,
            _c: Default::default(),
        }
    }

    pub fn header(&self) -> &H {
        &self.h
    }

    pub fn read_data<T: TypeShape>(&mut self, v: Version) -> anyhow::Result<T> {
        C::decode(v, &mut self.i)
    }
}

unsafe impl<C: Codec, H: TypeShape> Send for InboundFrame<C, H> {}

unsafe impl<C: Codec, H: TypeShape> Sync for InboundFrame<C, H> {}
