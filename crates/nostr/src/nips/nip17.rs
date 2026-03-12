// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-17: Private Direct Message
//!
//! <https://github.com/nostr-protocol/nips/blob/master/17.md>

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::types::url;
use crate::{Event, RelayUrl};

/// NIP-17 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Url error
    Url(url::Error),
    /// Missing tag kind
    MissingTagKind,
    /// Missing relay URL
    MissingRelayUrl,
    /// Unknown tag
    UnknownTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingRelayUrl => f.write_str("Missing relay URL"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

impl From<url::Error> for Error {
    fn from(e: url::Error) -> Self {
        Self::Url(e)
    }
}

/// Standardized NIP-17 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/17.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip17Tag {
    /// Relay
    ///
    /// `["relay", <relay URL>]`
    Relay(RelayUrl),
}

impl TagCodec for Nip17Tag {
    type Error = Error;

    /// Parse NIP-17 standardized tag
    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        // Take iterator
        let mut iter = tag.into_iter();

        // Extract first value
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        // Match kind
        match kind.as_ref() {
            // Parse as "relay" tag
            "relay" => {
                let url: RelayUrl = parse_relay_tag(iter)?;
                Ok(Self::Relay(url))
            }
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Relay(url) => {
                let tag: Vec<String> = vec![String::from("relay"), url.to_string()];
                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip17Tag);

fn parse_relay_tag<T, S>(mut iter: T) -> Result<RelayUrl, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse relay URL (index 1)
    let relay_url: S = iter.next().ok_or(Error::MissingRelayUrl)?;
    let relay_url: RelayUrl = RelayUrl::parse(relay_url.as_ref())?;

    Ok(relay_url)
}

/// Extracts the relay list
///
/// This function doesn't verify if the event kind is [`Kind::InboxRelays`](crate::Kind::InboxRelays)!
pub fn extract_relay_list(event: &Event) -> impl Iterator<Item = RelayUrl> + '_ {
    event
        .tags
        .iter()
        .filter_map(|tag| match Nip17Tag::parse(tag.as_slice()) {
            Ok(Nip17Tag::Relay(url)) => Some(url),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_tag() {
        let tag: Vec<String> = Vec::new();
        let err = Nip17Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingTagKind);
    }

    #[test]
    fn test_non_existing_tag() {
        let tag = vec!["p"];
        let err = Nip17Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::UnknownTag);
    }

    #[test]
    fn test_standardized_relay_tag() {
        let relay = RelayUrl::parse("wss://relay.damus.io").unwrap();
        let tag = vec!["relay".to_string(), relay.to_string()];

        let parsed = Nip17Tag::parse(&tag).unwrap();
        assert_eq!(parsed, Nip17Tag::Relay(relay.clone()));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_try_from_tag() {
        let relay = RelayUrl::parse("wss://relay.damus.io").unwrap();
        let tag = Tag::parse(["relay", "wss://relay.damus.io"]).unwrap();

        let parsed = Nip17Tag::try_from(&tag).unwrap();
        assert_eq!(parsed, Nip17Tag::Relay(relay));
    }

    #[test]
    fn test_into_tag() {
        let relay = RelayUrl::parse("wss://relay.damus.io").unwrap();
        let standardized = Nip17Tag::Relay(relay);

        assert_eq!(
            Tag::from(&standardized),
            Tag::parse(["relay", "wss://relay.damus.io"]).unwrap()
        );
        assert_eq!(
            Tag::from(standardized),
            Tag::parse(["relay", "wss://relay.damus.io"]).unwrap()
        );
    }

    #[test]
    fn test_missing_relay_url() {
        let tag = vec!["relay"];
        let err = Nip17Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingRelayUrl);
    }
}
