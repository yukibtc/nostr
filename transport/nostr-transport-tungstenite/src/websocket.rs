//! WebSocket transport

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
#[cfg(feature = "rustls")]
use std::sync::Arc;
use std::task::{Context, Poll};

use async_tungstenite::tungstenite;
use async_tungstenite::tungstenite::client::IntoClientRequest;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use futures::{Sink, Stream};
#[cfg(feature = "rustls")]
use futures_rustls::TlsConnector;
use nostr_runtime::prelude::*;
use nostr_transport::prelude::*;
#[cfg(feature = "rustls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "rustls")]
use rustls::{ClientConfig, RootCertStore};
#[cfg(feature = "rustls")]
use webpki_roots::TLS_SERVER_ROOTS;

/// Proxy target
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProxyTarget {
    /// All relays
    #[default]
    All,
    /// Only onion relays
    Onion,
}

/// Tungstenite websocket transport
#[derive(Debug, Default)]
pub struct TungsteniteWebSocketTransport {
    runtime: Option<Arc<dyn NostrRuntime>>,
    proxy: Option<SocketAddr>,
    proxy_target: ProxyTarget,
}

impl TungsteniteWebSocketTransport {
    /// Set a runtime
    #[inline]
    pub fn runtime(mut self, runtime: Arc<dyn NostrRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set a proxy
    pub fn proxy(mut self, proxy: SocketAddr, target: ProxyTarget) -> Self {
        self.proxy = Some(proxy);
        self.proxy_target = target;
        self
    }

    fn get_runtime(&self) -> Result<&Arc<dyn NostrRuntime>, TransportError> {
        match &self.runtime {
            Some(runtime) => Ok(runtime),
            None => {
                global::runtime().ok_or_else(|| TransportError::backend("no runtime installed"))
            }
        }
    }
}

impl NostrWebSocketTransport for TungsteniteWebSocketTransport {
    #[inline]
    fn support_ping(&self) -> bool {
        true
    }

    fn connect<'a>(
        &'a self,
        url: &'a RelayUrl,
    ) -> BoxedFuture<'a, Result<WebSocketStream, TransportError>> {
        Box::pin(async move {
            let stream: BoxedIoStream = self.connect_stream(self.get_runtime()?, url).await?;
            let request = url
                .as_str()
                .into_client_request()
                .map_err(TransportError::backend)?;

            let (stream, _) = async_tungstenite::client_async(request, stream)
                .await
                .map_err(TransportError::backend)?;

            Ok(WebSocketStream::new(TransportWebSocket(stream)))
        })
    }
}

impl TungsteniteWebSocketTransport {
    async fn connect_stream(
        &self,
        runtime: &Arc<dyn NostrRuntime>,
        url: &RelayUrl,
    ) -> Result<BoxedIoStream, TransportError> {
        let host = url
            .host_str()
            .ok_or_else(|| TransportError::backend("missing relay host"))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| TransportError::backend("missing relay port"))?;

        let tcp_stream = self.connect_tcp(runtime, url, host, port).await?;

        if url.scheme().is_secure() {
            return self.connect_tls(host, tcp_stream).await;
        }

        Ok(tcp_stream)
    }

    async fn connect_tcp(
        &self,
        runtime: &Arc<dyn NostrRuntime>,
        _url: &RelayUrl,
        host: &str,
        port: u16,
    ) -> Result<BoxedIoStream, TransportError> {
        {
            if let Some(proxy) = self.proxy {
                if self.should_use_proxy(_url) {
                    return connect_via_socks(runtime, proxy, host, port).await;
                }
            }
        }

        Ok(runtime
            .tcp_connect(TcpStreamAddr::HostAndPort { host, port })
            .await?)
    }

    async fn connect_tls(
        &self,
        host: &str,
        tcp_stream: BoxedIoStream,
    ) -> Result<BoxedIoStream, TransportError> {
        #[cfg(feature = "rustls")]
        {
            let server_name = match host.parse::<IpAddr>() {
                Ok(ip) => ServerName::IpAddress(ip.into()),
                Err(_) => ServerName::try_from(host.to_string())
                    .map_err(|_| TransportError::backend("invalid dns name"))?,
            };

            let config = rustls_config();
            let connector: TlsConnector = TlsConnector::from(Arc::new(config));
            let stream = connector
                .connect(server_name, tcp_stream)
                .await
                .map_err(TransportError::backend)?;

            return Ok(Box::pin(stream));
        }

        #[cfg(not(feature = "rustls"))]
        {
            let _ = host;
            let _ = tcp_stream;
            Err(TransportError::backend(
                "TLS support requires the rustls feature",
            ))
        }
    }

    #[inline]
    fn should_use_proxy(&self, url: &RelayUrl) -> bool {
        match self.proxy_target {
            ProxyTarget::All => true,
            ProxyTarget::Onion => url.is_onion(),
        }
    }
}

#[cfg(feature = "rustls")]
fn rustls_config() -> ClientConfig {
    ensure_rustls_provider();

    let mut roots = RootCertStore::empty();
    roots.extend(TLS_SERVER_ROOTS.iter().cloned());

    ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
}

#[cfg(feature = "rustls")]
fn ensure_rustls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

async fn connect_via_socks(
    runtime: &Arc<dyn NostrRuntime>,
    proxy: SocketAddr,
    host: &str,
    port: u16,
) -> Result<BoxedIoStream, TransportError> {
    let mut stream = runtime
        .tcp_connect(TcpStreamAddr::SocketAddr(proxy))
        .await?;

    stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut response = [0u8; 2];
    stream.read_exact(&mut response).await?;
    if response != [0x05, 0x00] {
        return Err(TransportError::IO(io::Error::new(
            io::ErrorKind::Other,
            "socks5 proxy does not allow no-auth method",
        )));
    }

    let mut request = Vec::with_capacity(32);
    request.extend_from_slice(&[0x05, 0x01, 0x00]);

    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ipv4)) => {
            request.push(0x01);
            request.extend_from_slice(&ipv4.octets());
        }
        Ok(IpAddr::V6(ipv6)) => {
            request.push(0x04);
            request.extend_from_slice(&ipv6.octets());
        }
        Err(_) => {
            let host_bytes = host.as_bytes();
            if host_bytes.len() > u8::MAX as usize {
                return Err(TransportError::IO(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "socks5 host name too long",
                )));
            }
            request.push(0x03);
            request.push(host_bytes.len() as u8);
            request.extend_from_slice(host_bytes);
        }
    }

    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await?;

    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 0x05 || header[1] != 0x00 {
        return Err(TransportError::IO(io::Error::new(
            io::ErrorKind::Other,
            format!("socks5 connect failed (rep={})", header[1]),
        )));
    }

    match header[3] {
        0x01 => {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).await?;
        }
        0x04 => {
            let mut buf = [0u8; 16];
            stream.read_exact(&mut buf).await?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut buf = vec![0u8; len[0] as usize];
            stream.read_exact(&mut buf).await?;
        }
        _ => {
            return Err(TransportError::IO(io::Error::new(
                io::ErrorKind::Other,
                "socks5 proxy replied with invalid address type",
            )));
        }
    }

    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).await?;

    Ok(stream)
}

struct TransportWebSocket<S>(async_tungstenite::WebSocketStream<S>);

impl<S> Stream for TransportWebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    type Item = Result<WebSocketMessage, TransportError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(Ok(message))) => {
                Poll::Ready(Some(Ok(TungsteniteMessage(message).into())))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(TransportError::backend(err)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> Sink<WebSocketMessage> for TransportWebSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    type Error = TransportError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_ready(cx)
            .map_err(TransportError::backend)
    }

    fn start_send(mut self: Pin<&mut Self>, item: WebSocketMessage) -> Result<(), Self::Error> {
        let item: TungsteniteMessage = item.into();
        Pin::new(&mut self.0)
            .start_send(item.0)
            .map_err(TransportError::backend)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_flush(cx)
            .map_err(TransportError::backend)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.0)
            .poll_close(cx)
            .map_err(TransportError::backend)
    }
}

struct TungsteniteCloseFrame(tungstenite::protocol::CloseFrame);

impl From<TungsteniteCloseFrame> for WebSocketCloseFrame {
    fn from(value: TungsteniteCloseFrame) -> Self {
        Self {
            code: value.0.code.into(),
            reason: unsafe { Utf8Bytes::from_bytes_unchecked(value.0.reason.into()) },
        }
    }
}

impl From<WebSocketCloseFrame> for TungsteniteCloseFrame {
    fn from(value: WebSocketCloseFrame) -> Self {
        Self(tungstenite::protocol::CloseFrame {
            code: value.code.into(),
            reason: unsafe { tungstenite::Utf8Bytes::from_bytes_unchecked(value.reason.into()) },
        })
    }
}

struct TungsteniteMessage(tungstenite::Message);

impl From<TungsteniteMessage> for WebSocketMessage {
    fn from(message: TungsteniteMessage) -> Self {
        match message.0 {
            tungstenite::Message::Text(text) => unsafe {
                Self::Text(Utf8Bytes::from_bytes_unchecked(text.into()))
            },
            tungstenite::Message::Binary(data) => Self::Binary(data),
            tungstenite::Message::Ping(data) => Self::Ping(data),
            tungstenite::Message::Pong(data) => Self::Pong(data),
            tungstenite::Message::Close(f) => {
                Self::Close(f.map(|f| TungsteniteCloseFrame(f).into()))
            }
            tungstenite::Message::Frame(_) => unreachable!(),
        }
    }
}

impl From<WebSocketMessage> for TungsteniteMessage {
    fn from(message: WebSocketMessage) -> Self {
        match message {
            WebSocketMessage::Text(text) => unsafe {
                let bytes = tungstenite::Utf8Bytes::from_bytes_unchecked(text.into());
                TungsteniteMessage(tungstenite::Message::Text(bytes))
            },
            WebSocketMessage::Binary(data) => {
                TungsteniteMessage(tungstenite::Message::Binary(data.into()))
            }
            WebSocketMessage::Ping(data) => {
                TungsteniteMessage(tungstenite::Message::Ping(data.into()))
            }
            WebSocketMessage::Pong(data) => {
                TungsteniteMessage(tungstenite::Message::Pong(data.into()))
            }
            WebSocketMessage::Close(f) => {
                TungsteniteMessage(tungstenite::Message::Close(f.map(|f| {
                    let f: TungsteniteCloseFrame = f.into();
                    f.0
                })))
            }
        }
    }
}
