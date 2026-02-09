use std::any::Any;
use std::fmt::Debug;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;

use futures_io::{AsyncRead, AsyncWrite};

use crate::future::BoxedFuture;

/// [`AsyncRead`] + [`AsyncWrite`]
pub trait AsyncReadWrite: AsyncRead + AsyncWrite {}

impl<T> AsyncReadWrite for T where T: AsyncRead + AsyncWrite + ?Sized {}

/// Boxed I/O stream
pub type BoxedIoStream = Pin<Box<dyn AsyncReadWrite + Send>>;

pub enum TcpStreamAddr<'a> {
    SocketAddr(SocketAddr),
    HostAndPort { host: &'a str, port: u16 },
}

/// Nostr runtime spawn
pub trait NostrRuntimeTcpStream: Any + Debug + Send + Sync {
    /// Creates a TCP connection to the specified address.
    fn tcp_connect<'a>(
        &self,
        addr: TcpStreamAddr<'a>,
    ) -> BoxedFuture<'a, Result<BoxedIoStream, io::Error>>;
}
