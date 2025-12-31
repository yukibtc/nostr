// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! Stream

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_utility::futures_util::Stream;
use tokio::sync::mpsc::Receiver;

use crate::relay::ReqExitPolicy;

/// Boxed stream
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
/// Boxed stream
#[cfg(target_arch = "wasm32")]
pub type BoxedStream<T> = Pin<Box<dyn Stream<Item = T>>>;

#[derive(Debug)]
pub(crate) struct ReceiverStream<T> {
    inner: Receiver<T>,
}

impl<T> ReceiverStream<T> {
    #[inline]
    pub(crate) fn new(recv: Receiver<T>) -> Self {
        Self { inner: recv }
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

/// Event stream request
#[derive(Debug)]
#[must_use = "does nothing unless you `.await`!"]
pub struct EventStreamRequest<'a, T, F> {
    pub(crate) obj: &'a T,
    pub(crate) filters: F,
    pub(crate) timeout: Duration,
    pub(crate) policy: ReqExitPolicy,
}

impl <'a, T, F> EventStreamRequest<'a, T, F> {
    pub(crate) fn new(obj: &'a T, filters: F) -> Self {
        Self {
            obj,
            filters,
            timeout: Duration::from_secs(60),
            policy: ReqExitPolicy::default()
        }
    }

    /// Set a timeout (default: 60 sec).
    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set request exit policy (default: [`ReqExitPolicy::ExitOnEOSE`]).
    #[inline]
    pub fn policy(mut self, policy: ReqExitPolicy) -> Self {
        self.policy = policy;
        self
    }
}
