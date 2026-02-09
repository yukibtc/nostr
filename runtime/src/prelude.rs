//! Prelude

#![allow(unknown_lints)]
#![allow(ambiguous_glob_reexports)]
#![doc(hidden)]

pub use crate::global;
pub use crate::net::{self, *};
pub use crate::runtime::{self, *};
pub use crate::spawn::{self, *};
pub use crate::time::{self, *};
