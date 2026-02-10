//! WebSocket transport

use std::any::Any;
use std::fmt::{self, Debug};
use std::pin::Pin;
use std::str;

use futures::{Sink, Stream, StreamExt};
use nostr::util::BoxedFuture;
use nostr::RelayUrl;

use super::error::TransportError;
use crate::bytes::{Bytes, Utf8Bytes};

/// WebSocket transport sink
#[cfg(not(target_arch = "wasm32"))]
pub type BoxWebSocketSink = Pin<Box<dyn Sink<WebSocketMessage, Error = TransportError> + Send>>;
/// WebSocket transport sink
#[cfg(target_arch = "wasm32")]
pub type BoxWebSocketSink = Pin<Box<dyn Sink<WebSocketMessage, Error = TransportError>>>;
/// WebSocket transport stream
#[cfg(not(target_arch = "wasm32"))]
pub type BoxWebSocketStream =
    Pin<Box<dyn Stream<Item = Result<WebSocketMessage, TransportError>> + Send>>;
/// Boxed stream
#[cfg(target_arch = "wasm32")]
pub type BoxWebSocketStream = Pin<Box<dyn Stream<Item = Result<WebSocketMessage, TransportError>>>>;

/// WebSocket close frame
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WebSocketCloseFrame {
    /// The reason as a code.
    pub code: u16,
    /// The reason as text string.
    pub reason: Utf8Bytes,
}

/// An enum representing the various forms of a WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WebSocketMessage {
    /// A text WebSocket message
    Text(Utf8Bytes),
    /// A binary WebSocket message
    Binary(Bytes),
    /// A ping message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Ping(Bytes),
    /// A pong message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Pong(Bytes),
    /// A close message with the optional close frame.
    Close(Option<WebSocketCloseFrame>),
}

impl fmt::Display for WebSocketMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(string) = self.as_text() {
            write!(f, "{string}")
        } else {
            write!(f, "Binary Data<length={}>", self.len())
        }
    }
}

impl WebSocketMessage {
    /// Get the length of the WebSocket message.
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Text(string) => string.len(),
            Self::Binary(data) => data.len(),
            Self::Ping(data) => data.len(),
            Self::Pong(data) => data.len(),
            Self::Close(data) => data.as_ref().map(|d| d.reason.len()).unwrap_or(0),
        }
    }

    /// Attempt to get a &str from the WebSocket message,
    /// this will try to convert binary data to utf8.
    fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(string) => Some(string.as_str()),
            Self::Binary(data) => str::from_utf8(data).ok(),
            Self::Ping(data) | Self::Pong(data) => str::from_utf8(data).ok(),
            Self::Close(None) => Some(""),
            Self::Close(Some(frame)) => Some(&frame.reason),
        }
    }
}

/// WebSocket stream
pub struct WebSocketStream {
    pub(crate) tx: BoxWebSocketSink,
    pub(crate) rx: BoxWebSocketStream,
}

impl WebSocketStream {
    /// Construct a new stream
    pub fn new<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<WebSocketMessage, TransportError>>
            + Sink<WebSocketMessage, Error = TransportError>
            + Send
            + 'static,
    {
        let (tx, rx) = stream.split();
        Self {
            tx: Box::pin(tx),
            rx: Box::pin(rx),
        }
    }

    /// Split stream
    #[inline]
    pub fn split(self) -> (BoxWebSocketSink, BoxWebSocketStream) {
        (self.tx, self.rx)
    }
}

/// WebSocket transport
pub trait NostrWebSocketTransport: Any + Debug + Send + Sync {
    /// Whether supports ping/pong
    fn support_ping(&self) -> bool;

    /// Connect via WebSocket
    fn connect<'a>(
        &'a self,
        url: &'a RelayUrl,
    ) -> BoxedFuture<'a, Result<WebSocketStream, TransportError>>;
}
