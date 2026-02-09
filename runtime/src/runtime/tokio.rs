//! Tokio runtime implementation

use std::io;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::runtime::{Handle, Runtime, TryCurrentError};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::future::BoxedFuture;
use crate::net::{BoxedIoStream, NostrRuntimeTcpStream, TcpStreamAddr};
use crate::prelude::BoxedBlockingOutput;
use crate::spawn::{
    BoxedBlockingTask, NostrRuntimeSpawn, NostrRuntimeSpawnBlockingTask, SpawnBlockingTaskError,
};
use crate::time::NostrRuntimeTimer;

#[derive(Debug)]
enum InnerRuntime {
    Runtime(Runtime),
    Handle(Handle),
}

impl InnerRuntime {
    #[inline]
    fn handle(&self) -> &Handle {
        match self {
            InnerRuntime::Runtime(r) => r.handle(),
            InnerRuntime::Handle(h) => h,
        }
    }
}

/// Tokio runtime handle
#[derive(Debug)]
pub struct TokioRuntime(InnerRuntime);

impl TokioRuntime {
    /// Create a **new** tokio [`Runtime`].
    ///
    /// Use [`TokioRuntime::current`] or [`TokioRuntime::try_current`] to construct this from an **already existing** runtime.
    pub fn new() -> io::Result<Self> {
        let runtime: Runtime = Runtime::new()?;
        Ok(Self(InnerRuntime::Runtime(runtime)))
    }

    /// Construct runtime from current handle
    ///
    /// Use [`TokioRuntime::new`] to create a new runtime.
    ///
    /// # Panics
    ///
    /// This will panic if called outside the context of a Tokio runtime.
    pub fn current() -> Self {
        Self::from(Handle::current())
    }

    /// Construct runtime from current handle
    ///
    /// Use [`TokioRuntime::new`] to create a new runtime.
    pub fn try_current() -> Result<Self, TryCurrentError> {
        Handle::try_current().map(Self::from)
    }
}

impl From<Handle> for TokioRuntime {
    #[inline]
    fn from(handle: Handle) -> Self {
        Self(InnerRuntime::Handle(handle))
    }
}

impl NostrRuntimeSpawn for TokioRuntime {
    fn spawn_boxed(&self, future: BoxedFuture<'static, ()>) {
        let _join_handle = self.0.handle().spawn(future);
    }
}

impl NostrRuntimeSpawnBlockingTask for TokioRuntime {
    fn spawn_blocking_task_boxed(
        &self,
        task: BoxedBlockingTask,
    ) -> BoxedFuture<Result<BoxedBlockingOutput, SpawnBlockingTaskError>> {
        Box::pin(async move {
            self.0
                .handle()
                .spawn_blocking(move || task())
                .await
                .map_err(SpawnBlockingTaskError::new)
        })
    }
}

impl NostrRuntimeTimer for TokioRuntime {
    fn sleep(&self, duration: Duration) -> BoxedFuture<'static, ()> {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
        })
    }
}

impl NostrRuntimeTcpStream for TokioRuntime {
    fn tcp_connect<'a>(
        &self,
        addr: TcpStreamAddr<'a>,
    ) -> BoxedFuture<'a, Result<BoxedIoStream, io::Error>> {
        Box::pin(async move {
            let stream = match addr {
                TcpStreamAddr::SocketAddr(addr) => TcpStream::connect(addr).await?,
                TcpStreamAddr::HostAndPort { host, port } => {
                    TcpStream::connect((host, port)).await?
                }
            };
            let stream = stream.compat();
            Ok(Box::pin(stream) as BoxedIoStream)
        })
    }
}
