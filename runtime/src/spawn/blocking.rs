use std::any::Any;
use std::fmt::{self, Debug};

use crate::future::BoxedFuture;

/// A boxed blocking output.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub type BoxedBlockingOutput = Box<dyn Any + Send>;

/// A boxed blocking output.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub type BoxedBlockingOutput = Box<dyn Any>;

/// A boxed blocking task.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub type BoxedBlockingTask = Box<dyn FnOnce() -> BoxedBlockingOutput + Send>;

/// A boxed blocking task.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub type BoxedBlockingTask = Box<dyn FnOnce() -> BoxedBlockingOutput>;

/// Spawn blocking task error
#[derive(Debug)]
pub struct SpawnBlockingTaskError {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    e: Box<dyn std::error::Error + Send + Sync>,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    e: Box<dyn std::error::Error>,
}

impl std::error::Error for SpawnBlockingTaskError {}

impl fmt::Display for SpawnBlockingTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.e)
    }
}

impl SpawnBlockingTaskError {
    /// New spawn blocking task error
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self { e: error.into() }
    }

    /// New spawn blocking task error
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        Self { e: error.into() }
    }
}

/// Spawn blocking
pub trait NostrRuntimeSpawnBlockingTask: Any + Debug + Send + Sync {
    /// Spawn blocking boxed
    fn spawn_blocking_task_boxed(
        &self,
        task: BoxedBlockingTask,
    ) -> BoxedFuture<Result<BoxedBlockingOutput, SpawnBlockingTaskError>>;
}

/// Spawn blocking extension helpers.
pub trait NostrRuntimeSpawnBlockingTaskExt: NostrRuntimeSpawnBlockingTask {
    /// Spawn blocking and await the output.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn spawn_blocking_task<F, TOut>(
        &self,
        task: F,
    ) -> BoxedFuture<Result<TOut, SpawnBlockingTaskError>>
    where
        F: FnOnce() -> TOut + Send + 'static,
        TOut: Send + 'static,
    {
        let future = self
            .spawn_blocking_task_boxed(Box::new(move || Box::new(task()) as BoxedBlockingOutput));
        Box::pin(exec_and_downcast(future))
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn spawn_blocking_task<F, TOut>(
        &self,
        task: F,
    ) -> BoxedFuture<'static, Result<TOut, SpawnBlockingTaskError>>
    where
        F: FnOnce() -> TOut + 'static,
        TOut: 'static,
    {
        let future = self
            .spawn_blocking_task_boxed(Box::new(move || Box::new(task()) as BoxedBlockingOutput));
        Box::pin(exec_and_downcast(future))
    }
}

impl<T> NostrRuntimeSpawnBlockingTaskExt for T where T: NostrRuntimeSpawnBlockingTask + ?Sized {}

async fn exec_and_downcast<TOut>(
    future: BoxedFuture<'_, Result<BoxedBlockingOutput, SpawnBlockingTaskError>>,
) -> Result<TOut, SpawnBlockingTaskError>
where
    TOut: 'static,
{
    let output = future.await?;
    Ok(*output
        .downcast()
        .map_err(|_| SpawnBlockingTaskError::new("output type mismatch"))?)
}
