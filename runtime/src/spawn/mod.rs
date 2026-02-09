//! Nostr runtime spawn traits

use std::any::Any;
use std::fmt::Debug;
use std::future::Future;

mod blocking;

pub use self::blocking::*;
use crate::future::BoxedFuture;

/// Nostr runtime spawn
pub trait NostrRuntimeSpawn: Any + Debug + Send + Sync {
    /// Spawn boxed
    fn spawn_boxed(&self, future: BoxedFuture<'static, ()>);

    /// Spawn
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn spawn<F>(&self, future: F)
    where
        Self: Sized,
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn_boxed(Box::pin(future))
    }

    /// Spawn
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn spawn<F>(&self, future: F)
    where
        Self: Sized,
        F: Future<Output = ()> + 'static,
    {
        self.spawn_boxed(Box::pin(future))
    }
}
