// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-89: Recommended Application Handlers
//!
//! <https://github.com/nostr-protocol/nips/blob/master/89.md>

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use super::nip01::Coordinate;
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::types::url::RelayUrl;

const CLIENT: &str = "client";

/// NIP-89 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Missing tag kind
    MissingTagKind,
    /// Missing value
    MissingClientName,
    /// Unknown tag
    UnknownTag,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingClientName => f.write_str("Missing alt value"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

/// Standardized NIP-89 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/89.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip89Tag {
    /// `client` tag
    Client {
        /// Client name
        name: String,
        /// Client address and optional hint
        address: Option<(Coordinate, Option<RelayUrl>)>,
    },
}

impl TagCodec for Nip89Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            CLIENT => parse_client_tag(iter),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Client { name, address } => {
                let mut tag: Vec<String> = vec![CLIENT.to_string(), name.clone()];

                match address {
                    Some((coordinate, Some(hint))) => {
                        tag.reserve_exact(2);
                        tag.push(coordinate.to_string());
                        tag.push(hint.to_string());
                    }
                    Some((coordinate, None)) => {
                        tag.push(coordinate.to_string());
                    }
                    _ => {}
                }

                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip89Tag);

fn parse_client_tag<T, S>(mut iter: T) -> Result<Nip89Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Possible cases:
    // - ["client", "My Client"]
    // - ["client", "My Client", "31990:app1-pubkey:<d-identifier>"]
    // - ["client", "My Client", "31990:app1-pubkey:<d-identifier>", "wss://relay1"]

    let name: S = iter.next().ok_or(Error::MissingClientName)?;
    let name: String = name.as_ref().to_string();

    let coordinate: Option<S> = iter.next();

    // Since the address is optional,
    // don't return an error if the coordinate or relay hint parsing fails.
    let address: Option<(Coordinate, Option<RelayUrl>)> = match coordinate {
        // Try to parse the coordinate
        Some(coordinate) => match Coordinate::parse(coordinate.as_ref()) {
            // Coordinate parsing success
            Ok(coordinate) => {
                let relay_url: Option<S> = iter.next();
                let relay_url: Option<RelayUrl> =
                    relay_url.and_then(|url| RelayUrl::parse(url.as_ref()).ok());
                Some((coordinate, relay_url))
            }
            // Failed to parse the coordinate
            Err(..) => None,
        },
        // Nothing to parse
        None => None,
    };

    Ok(Nip89Tag::Client { name, address })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_tag() {
        let tag = vec!["client", "voyage"];
        let parsed = Nip89Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip89Tag::Client {
                name: String::from("voyage"),
                address: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_client_tag_with_coordinate() {
        let tag = vec![
            "client",
            "voyage",
            "30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum",
        ];
        let parsed = Nip89Tag::parse(&tag).unwrap();

        assert_eq!(parsed, Nip89Tag::Client {name: String::from("voyage"), address: Some((Coordinate::parse("30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum").unwrap(), None))});
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_client_tag_with_coordinate_and_relay_hint() {
        let tag = vec![
            "client",
            "voyage",
            "30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum",
            "wss://relay.damus.io",
        ];
        let parsed = Nip89Tag::parse(&tag).unwrap();

        assert_eq!(parsed, Nip89Tag::Client {name: String::from("voyage"), address: Some((Coordinate::parse("30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum").unwrap(), Some(RelayUrl::parse("wss://relay.damus.io").unwrap())))});
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_client_tag_with_coordinate_and_empty_relay_hint() {
        let tag = vec![
            "client",
            "voyage",
            "30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum",
            "",
        ];
        let parsed = Nip89Tag::parse(&tag).unwrap();

        assert_eq!(parsed, Nip89Tag::Client {name: String::from("voyage"), address: Some((Coordinate::parse("30023:a695f6b60119d9521934a691347d9f78e8770b56da16bb255ee286ddf9fda919:ipsum").unwrap(), None))});
        assert_eq!(
            parsed.to_tag(),
            Tag::parse(tag[..=2].iter().copied()).unwrap()
        ); // The empty relay-hint is not serialized
    }
}
