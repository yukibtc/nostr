//! Nostr runtime timer

use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::future::BoxedFuture;

/// Timeout error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("timeout")
    }
}

impl std::error::Error for TimeoutError {}

struct Timeout<T> {
    future: BoxedFuture<'static, T>,
    sleep: BoxedFuture<'static, ()>,
}

impl<T> Future for Timeout<T> {
    type Output = Result<T, TimeoutError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        if let Poll::Ready(value) = this.future.as_mut().poll(cx) {
            return Poll::Ready(Ok(value));
        }

        if let Poll::Ready(()) = this.sleep.as_mut().poll(cx) {
            return Poll::Ready(Err(TimeoutError));
        }

        Poll::Pending
    }
}

/// Nostr Runtime Timer
pub trait NostrRuntimeTimer: Any + Debug + Send + Sync {
    /// Sleep
    fn sleep(&self, duration: Duration) -> BoxedFuture<'static, ()>;

    /// Timeout
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn timeout<F, T>(
        &self,
        duration: Duration,
        future: F,
    ) -> BoxedFuture<'static, Result<T, TimeoutError>>
    where
        Self: Sized,
        F: Future<Output = T> + Send + 'static,
        T: 'static,
    {
        let sleep = self.sleep(duration);
        let future = Box::pin(future);

        Box::pin(Timeout { future, sleep })
    }

    /// Timeout
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn timeout<F, T>(
        &self,
        duration: Duration,
        future: F,
    ) -> BoxedFuture<'static, Result<T, TimeoutError>>
    where
        Self: Sized,
        F: Future<Output = T> + 'static,
        T: 'static,
    {
        let sleep = self.sleep(duration);
        let future = Box::pin(future);

        Box::pin(Timeout { future, sleep })
    }
}
