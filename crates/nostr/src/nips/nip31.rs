// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-31: Dealing with unknown event kinds
//!
//! <https://github.com/nostr-protocol/nips/blob/master/31.md>

use alloc::string::{String, ToString};
use alloc::vec;
use core::fmt;

use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};

const ALT: &str = "alt";

/// NIP-31 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Missing tag kind
    MissingTagKind,
    /// Missing value
    MissingAltValue,
    /// Unknown tag
    UnknownTag,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingAltValue => f.write_str("Missing alt value"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

/// Standardized NIP-31 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/31.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip31Tag {
    /// `alt` tag
    Alt(String),
}

impl TagCodec for Nip31Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            ALT => parse_alt_tag(iter),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Alt(value) => Tag::new(vec![String::from(ALT), value.clone()]),
        }
    }
}

impl_tag_codec_conversions!(Nip31Tag);

fn parse_alt_tag<T, S>(mut iter: T) -> Result<Nip31Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let value: S = iter.next().ok_or(Error::MissingAltValue)?;
    let value: String = value.as_ref().to_string();

    Ok(Nip31Tag::Alt(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nip31_alt_tag() {
        let tag = vec!["alt", "Something"];
        let parsed = Nip31Tag::parse(&tag).unwrap();

        assert_eq!(parsed, Nip31Tag::Alt(String::from("Something")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_invalid_alt_tag_missing_value() {
        let tag = vec!["alt"];
        assert_eq!(Nip31Tag::parse(&tag).unwrap_err(), Error::MissingAltValue);
    }
}
