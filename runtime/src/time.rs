//! Nostr runtime time

use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::future::BoxedFuture;

/// Timeout error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError {
    _priv: (),
}

impl std::error::Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("timeout")
    }
}

impl TimeoutError {
    #[inline]
    fn new() -> Self {
        Self { _priv: () }
    }
}

/// Nostr Runtime Timer
pub trait NostrRuntimeTimer: Any + Debug + Send + Sync {
    /// Sleep
    fn sleep(&self, duration: Duration) -> BoxedFuture<'_, ()>;
}

/// Extensions for the [`NostrRuntimeTimer`] trait.
pub trait NostrRuntimeTimerExt: NostrRuntimeTimer {
    /// Timeout
    fn timeout<'a, F, T>(
        &'a self,
        duration: Duration,
        future: F,
    ) -> BoxedFuture<'a, Result<T, TimeoutError>>
    where
        F: Future<Output = T> + Send + 'a,
        T: 'a,
    {
        Box::pin(async move {
            let sleep = self.sleep(duration);
            let future = Box::pin(future);
            Timeout { future, sleep }.await
        })
    }
}

impl<T> NostrRuntimeTimerExt for T where T: NostrRuntimeTimer + ?Sized {}

struct Timeout<'a, T> {
    sleep: BoxedFuture<'a, ()>,
    future: BoxedFuture<'a, T>,
}

impl<T> Future for Timeout<'_, T> {
    type Output = Result<T, TimeoutError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        if let Poll::Ready(value) = this.future.as_mut().poll(cx) {
            return Poll::Ready(Ok(value));
        }

        if let Poll::Ready(()) = this.sleep.as_mut().poll(cx) {
            return Poll::Ready(Err(TimeoutError::new()));
        }

        Poll::Pending
    }
}
