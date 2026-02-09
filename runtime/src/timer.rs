//! Nostr runtime timer

use std::any::Any;
use std::fmt::Debug;
use std::time::Duration;

use crate::future::BoxedFuture;

/// Nostr Runtime Timer
pub trait NostrRuntimeTimer: Any + Debug + Send + Sync {
    /// Sleep
    fn sleep(&self, duration: Duration) -> BoxedFuture<'_, ()>;
}
