use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use nostr::util::BoxedFuture;
use nostr_runtime::prelude::*;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeWrapper {
    runtime: Arc<dyn NostrRuntime>,
}

impl Deref for RuntimeWrapper {
    type Target = Arc<dyn NostrRuntime>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl RuntimeWrapper {
    #[inline]
    pub(crate) fn new(runtime: Arc<dyn NostrRuntime>) -> Self {
        Self { runtime }
    }

    pub(crate) async fn maybe_timeout<F, T>(
        &self,
        duration: Option<Duration>,
        future: F,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = T> + Send,
    {
        match duration {
            Some(duration) => self.runtime.timeout(duration, future).await,
            None => Ok(future.await),
        }
    }
}
