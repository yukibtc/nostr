//! Nostr Runtime

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]
#![warn(clippy::large_futures)]

mod future;
pub mod global;
pub mod net;
pub mod prelude;
pub mod runtime;
pub mod spawn;
pub mod time;
