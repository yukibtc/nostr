// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! Stream

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_utility::futures_util::Stream;
use tokio::sync::mpsc::Receiver;
use nostr::{Filter, RelayUrl};

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

#[derive(Debug, Clone)]
pub enum EventStreamTarget {
    /// Broadcast to all relays
    Broadcast(Vec<Filter>),
    /// Send to specific relays
    Targeted(HashMap<RelayUrl, Vec<Filter>>),
}

impl From<Filter> for EventStreamTarget {
    fn from(filter: Filter) -> Self {
        Self::Broadcast(vec![filter])
    }
}

impl From<Vec<Filter>> for EventStreamTarget {
    fn from(filters: Vec<Filter>) -> Self {
        Self::Broadcast(filters)
    }
}

impl From<HashMap<RelayUrl, Vec<Filter>>> for EventStreamTarget {
    fn from(targets: HashMap<RelayUrl, Vec<Filter>>) -> Self {
        Self::Targeted(targets)
    }
}

/// Event stream request
#[must_use = "does nothing unless you `.await`!"]
pub struct EventStreamRequest<'a, T> {
    pub(crate) obj: &'a T,
    pub(crate) preexec: Option<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>>,
    pub(crate) target: EventStreamTarget,
    pub(crate) timeout: Duration,
    pub(crate) policy: ReqExitPolicy,
}

impl <'a, T> EventStreamRequest<'a, T> {
    pub(crate) fn new<F>(obj: &'a T, target: F) -> Self
    where
        F: Into<EventStreamTarget>,
    {
        Self {
            obj,
            target: target.into(),
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
