use futures_util::FutureExt;
use hyper::rt::Executor;
use std::marker::PhantomPinned;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;

pub(super) struct Scope {
    i: AtomicUsize,
    n: Notify,
    _pin: PhantomPinned, // marker
}

#[derive(Clone)]
pub(super) struct ScopeRef {
    opaque: usize,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            i: AtomicUsize::new(1),
            n: Notify::new(),
            _pin: PhantomPinned,
        }
    }

    pub async fn join(&self) {
        if self.i.fetch_sub(1, Ordering::SeqCst) == 1 {
            return;
        }
        self.n.notified().await;
    }

    pub fn refer(self: Pin<&Self>) -> ScopeRef {
        ScopeRef {
            opaque: self.get_ref() as *const _ as usize,
        }
    }
}

impl ScopeRef {
    fn recover(&self) -> &Scope {
        unsafe { &*(self.opaque as *const Scope) }
    }

    pub fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        self.recover().i.fetch_add(1, Ordering::SeqCst);
        let refer = self.clone();
        tokio::spawn(async move {
            let _ = AssertUnwindSafe(task).catch_unwind().await;
            if refer.recover().i.fetch_sub(1, Ordering::SeqCst) == 1 {
                refer.recover().n.notify_one()
            }
        });
    }
}

impl<Fut> Executor<Fut> for ScopeRef
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        self.spawn(async move {
            let _ = fut.await;
        })
    }
}
