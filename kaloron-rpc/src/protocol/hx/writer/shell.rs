// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use super::super::facade::Facade;
use super::inner::{FrameWriterCore, OutboundFrameInner};
use bytes::Bytes;
use hyper::body::{Body, Frame};
use kaloron::{TypeShape, Version};
use kaloron_codec::Codec;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub struct FrameWriterBody {
    core: Option<Arc<Mutex<FrameWriterCore>>>,
}

impl FrameWriterBody {
    pub fn empty() -> Self {
        Self { core: None }
    }
}

impl Body for FrameWriterBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &self.core {
            None => Poll::Ready(None),
            Some(core) => {
                let mut core = core.lock().expect("frame writer core mutex poisoned");
                if let Some(bytes) = core.i.poll_frame() {
                    core.waiter = None;
                    Poll::Ready(Some(Ok(Frame::data(bytes))))
                } else if core.closed {
                    core.waiter = None;
                    Poll::Ready(None)
                } else {
                    let should_replace = match core.waiter.as_ref() {
                        Some(waiter) => !waiter.will_wake(cx.waker()),
                        None => true,
                    };
                    if should_replace {
                        core.waiter = Some(cx.waker().clone());
                    }
                    Poll::Pending
                }
            }
        }
    }
}

pub struct FrameWriter<C: Codec, H: TypeShape> {
    core: Arc<Mutex<FrameWriterCore>>,
    _marker: PhantomData<fn() -> (C, H)>,
}

impl<C: Codec, H: TypeShape> Drop for FrameWriter<C, H> {
    fn drop(&mut self) {
        let waiter = {
            let mut core = self.core.lock().expect("frame writer core mutex poisoned");
            core.closed = true;
            core.waiter.take()
        };

        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

impl<C: Codec, H: TypeShape> FrameWriter<C, H> {
    pub fn new(v: Version, facade: Facade) -> Self {
        Self {
            core: Arc::new(Mutex::new(FrameWriterCore::new(v, facade))),
            _marker: PhantomData,
        }
    }

    pub fn body(&self) -> FrameWriterBody {
        let mut core = self.core.lock().expect("frame writer core mutex poisoned");
        assert!(!core.body_created, "frame writer body already created");
        core.body_created = true;
        FrameWriterBody {
            core: Some(Arc::clone(&self.core)),
        }
    }

    pub fn frame<T: TypeShape>(&self, v: Version, h: H, d: T) -> anyhow::Result<()> {
        let waiter = {
            let mut core = self.core.lock().expect("frame writer core mutex poisoned");
            let builder = core.i.new_frame();
            let mut writer = OutboundFrameInner::new(builder, &core.facade);
            C::encode(core.v, &h, &mut writer)?;
            C::encode(v, &d, &mut writer)?;
            writer.finish()?;
            core.waiter.take()
        };

        if let Some(waiter) = waiter {
            waiter.wake();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::inner::OutboundFrameInner;
    use super::super::ring::RingGather;
    use super::*;
    use crate::protocol::hx::facade::Facade;
    use kaloron_codec::TextualCodec;
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Wake, Waker};

    #[derive(Default)]
    struct MockWakeCounter {
        wakes: AtomicUsize,
    }

    impl MockWakeCounter {
        fn count(&self) -> usize {
            self.wakes.load(Ordering::SeqCst)
        }
    }

    impl Wake for MockWakeCounter {
        fn wake(self: Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_frame_emits_length_prefix_before_payload() {
        let writer = FrameWriter::<TextualCodec, u8>::new(Version::zero(), Facade::empty());
        writer
            .frame(Version::zero(), 1u8, 2u8)
            .expect("frame encoding should succeed");
        let mut body = writer.body();
        drop(writer);

        let poll_waker = Waker::from(Arc::new(MockWakeCounter::default()));
        let mut cx = Context::from_waker(&poll_waker);

        let first = match Pin::new(&mut body).poll_frame(&mut cx) {
            Poll::Ready(Some(Ok(frame))) => frame.into_data().expect("expected data frame"),
            other => panic!("unexpected first poll result: {other:?}"),
        };
        assert_eq!(first.as_ref(), 8u32.to_le_bytes().as_slice());

        let second = match Pin::new(&mut body).poll_frame(&mut cx) {
            Poll::Ready(Some(Ok(frame))) => frame.into_data().expect("expected data frame"),
            other => panic!("unexpected second poll result: {other:?}"),
        };
        assert_eq!(second.as_ref(), b"u8:1u8:2");

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Ready(None)
        ));
    }

    #[test]
    fn test_pending_waiter_is_woken_when_frame_is_enqueued() {
        let writer = FrameWriter::<TextualCodec, u8>::new(Version::zero(), Facade::empty());
        let mut body = writer.body();
        let wake_counter = Arc::new(MockWakeCounter::default());
        let waker = Waker::from(wake_counter.clone());
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Pending
        ));
        assert_eq!(wake_counter.count(), 0);

        writer
            .frame(Version::zero(), 1u8, 2u8)
            .expect("frame encoding should succeed");
        assert_eq!(wake_counter.count(), 1);

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Ready(Some(Ok(_)))
        ));
    }

    #[test]
    fn test_pending_waiter_is_woken_from_another_thread_when_frame_is_enqueued() {
        let writer = FrameWriter::<TextualCodec, u8>::new(Version::zero(), Facade::empty());
        let mut body = writer.body();
        let wake_counter = Arc::new(MockWakeCounter::default());
        let waker = Waker::from(wake_counter.clone());
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Pending
        ));
        assert_eq!(wake_counter.count(), 0);

        let handle = std::thread::spawn(move || {
            writer
                .frame(Version::zero(), 1u8, 2u8)
                .expect("frame encoding should succeed");
        });
        handle.join().expect("frame thread should finish");
        assert_eq!(wake_counter.count(), 1);

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Ready(Some(Ok(_)))
        ));
    }

    #[test]
    fn test_pending_waiter_is_woken_and_stream_ends_when_last_writer_drops() {
        let writer = FrameWriter::<TextualCodec, u8>::new(Version::zero(), Facade::empty());
        let mut body = writer.body();
        let wake_counter = Arc::new(MockWakeCounter::default());
        let waker = Waker::from(wake_counter.clone());
        let mut cx = Context::from_waker(&waker);

        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Pending
        ));
        assert_eq!(wake_counter.count(), 0);

        drop(writer);
        assert_eq!(wake_counter.count(), 1);
        assert!(matches!(
            Pin::new(&mut body).poll_frame(&mut cx),
            Poll::Ready(None)
        ));
    }

    #[test]
    fn test_outbound_inner_flushes_full_chunks_and_trims_final_chunk() {
        let mut ring = RingGather::new();
        let payload = vec![b'x'; 1024 * 2 + 17];

        {
            let builder = ring.new_frame();
            let mut writer = OutboundFrameInner::new(builder, &Facade::empty());
            writer
                .write_all(&payload)
                .expect("payload write should fill all chunks");
            writer.finish().expect("finish should succeed");
        }

        let header = ring.poll_frame().expect("frame header should be queued");
        assert_eq!(
            header.as_ref(),
            (payload.len() as u32).to_le_bytes().as_slice()
        );

        let first = ring
            .poll_frame()
            .expect("first full payload chunk should exist");
        assert_eq!(first.len(), 1024);
        assert!(first.iter().all(|byte| *byte == b'x'));

        let second = ring
            .poll_frame()
            .expect("second full payload chunk should exist");
        assert_eq!(second.len(), 1024);
        assert!(second.iter().all(|byte| *byte == b'x'));

        let tail = ring.poll_frame().expect("tail payload chunk should exist");
        assert_eq!(tail.len(), 17);
        assert!(tail.iter().all(|byte| *byte == b'x'));

        assert_eq!(ring.poll_frame(), None);
    }
}
