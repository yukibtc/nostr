//! Prelude

#![allow(unknown_lints)]
#![allow(ambiguous_glob_reexports)]
#![doc(hidden)]

pub use nostr::prelude::*;

pub use crate::bytes::{self, *};
pub use crate::error::{self, *};
pub use crate::websocket::{self, *};
