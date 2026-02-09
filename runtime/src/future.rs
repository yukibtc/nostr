use std::future::Future;
use std::pin::Pin;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed future
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
