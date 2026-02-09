//! Runtimes

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "tokio")]
pub use self::tokio::*;
use crate::net::NostrRuntimeTcpStream;
use crate::spawn::{NostrRuntimeSpawn, NostrRuntimeSpawnBlockingTask};
use crate::time::NostrRuntimeTimer;

/// Nostr runtime
pub trait NostrRuntime:
    NostrRuntimeSpawn + NostrRuntimeSpawnBlockingTask + NostrRuntimeTimer + NostrRuntimeTcpStream
{
}

impl<T> NostrRuntime for T where
    T: NostrRuntimeSpawn
        + NostrRuntimeSpawnBlockingTask
        + NostrRuntimeTimer
        + NostrRuntimeTcpStream
{
}
