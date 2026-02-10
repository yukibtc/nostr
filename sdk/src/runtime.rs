use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use nostr_runtime::prelude::*;
use nostr_transport::prelude::*;
#[cfg(feature = "transport-tungstenite")]
use nostr_transport_tungstenite::prelude::*;

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

pub(crate) fn get_runtime(runtime: Option<Arc<dyn NostrRuntime>>) -> Option<Arc<dyn NostrRuntime>> {
    match runtime {
        Some(runtime) => Some(runtime),
        None => match global::runtime() {
            Some(runtime) => Some(runtime.clone()),
            None => {
                #[cfg(feature = "runtime-tokio")]
                match TokioRuntime::try_current() {
                    Ok(runtime) => Some(Arc::new(runtime)),
                    Err(_) => None,
                }
                #[cfg(not(feature = "runtime-tokio"))]
                None
            }
        },
    }
}

pub(crate) fn get_transport(
    runtime: &Arc<dyn NostrRuntime>,
    transport: Option<Arc<dyn NostrWebSocketTransport>>,
) -> Option<Arc<dyn NostrWebSocketTransport>> {
    match transport {
        Some(transport) => Some(transport),
        None => {
            #[cfg(feature = "transport-tungstenite")]
            {
                Some(Arc::new(
                    TungsteniteWebSocketTransport::default().runtime(runtime.clone()),
                ))
            }
            #[cfg(not(feature = "transport-tungstenite"))]
            None
        }
    }
}
