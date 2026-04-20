// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-18: Reposts
//!
//! <https://github.com/nostr-protocol/nips/blob/master/18.md>

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use super::util::take_and_parse_relay_hint;
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::event::{self};
use crate::nips::nip01::{self, Coordinate};
use crate::types::url;
use crate::{EventId, Kind, PublicKey, RelayUrl, key};

const EVENT: &str = "e";
const KIND: &str = "k";
const PUBLIC_KEY: &str = "p";
const QUOTE: &str = "q";

/// NIP-18 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Event error
    Event(event::Error),
    /// Keys error
    Keys(key::Error),
    /// NIP-01 error
    Nip01(nip01::Error),
    /// Relay URL error
    RelayUrl(url::Error),
    /// Missing tag kind
    MissingTagKind,
    /// Missing event ID
    MissingEventId,
    /// Missing public key
    MissingPublicKey,
    /// Missing kind
    MissingKind,
    /// Invalid kind
    InvalidKind,
    /// Unknown tag
    UnknownTag,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event(e) => e.fmt(f),
            Self::Keys(e) => e.fmt(f),
            Self::Nip01(e) => e.fmt(f),
            Self::RelayUrl(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingEventId => f.write_str("Missing event ID"),
            Self::MissingPublicKey => f.write_str("Missing public key"),
            Self::MissingKind => f.write_str("Missing kind"),
            Self::InvalidKind => f.write_str("Invalid kind"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

impl From<key::Error> for Error {
    fn from(e: key::Error) -> Self {
        Self::Keys(e)
    }
}

impl From<event::Error> for Error {
    fn from(e: event::Error) -> Self {
        Self::Event(e)
    }
}

impl From<nip01::Error> for Error {
    fn from(e: nip01::Error) -> Self {
        Self::Nip01(e)
    }
}

impl From<url::Error> for Error {
    fn from(e: url::Error) -> Self {
        Self::RelayUrl(e)
    }
}

/// Standardized NIP-18 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/18.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip18Tag {
    /// `e` tag
    Event {
        /// Event ID
        id: EventId,
        /// Relay hint
        relay_hint: Option<RelayUrl>,
    },
    /// `k` tag
    Kind(Kind),
    /// `p` tag
    PublicKey {
        /// Public key
        public_key: PublicKey,
        /// Relay hint
        relay_hint: Option<RelayUrl>,
    },
    /// `q` tag with event ID
    Quote {
        /// Event ID
        id: EventId,
        /// Relay hint
        relay_hint: Option<RelayUrl>,
        /// Public key hint
        public_key: Option<PublicKey>,
    },
    /// `q` tag with event coordinate
    QuoteAddress {
        /// Event coordinate
        coordinate: Coordinate,
        /// Relay hint
        relay_hint: Option<RelayUrl>,
    },
}

impl TagCodec for Nip18Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            EVENT => parse_e_tag(iter),
            KIND => parse_k_tag(iter),
            PUBLIC_KEY => parse_p_tag(iter),
            QUOTE => parse_q_tag(iter),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Event { id, relay_hint } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);
                tag.push(String::from(EVENT));
                tag.push(id.to_hex());

                if let Some(relay_hint) = relay_hint {
                    tag.push(relay_hint.to_string());
                }

                Tag::new(tag)
            }
            Self::Kind(kind) => Tag::new(vec![String::from(KIND), kind.as_u16().to_string()]),
            Self::PublicKey {
                public_key,
                relay_hint,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);
                tag.push(String::from(PUBLIC_KEY));
                tag.push(public_key.to_hex());

                if let Some(relay_hint) = relay_hint {
                    tag.push(relay_hint.to_string());
                }

                Tag::new(tag)
            }
            Self::Quote {
                id,
                relay_hint,
                public_key,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(
                    2 + relay_hint.is_some() as usize + public_key.is_some() as usize,
                );
                tag.push(String::from(QUOTE));
                tag.push(id.to_hex());

                match relay_hint {
                    Some(relay_hint) => tag.push(relay_hint.to_string()),
                    None => {
                        if public_key.is_some() {
                            tag.push(String::new());
                        }
                    }
                }

                if let Some(public_key) = public_key {
                    tag.push(public_key.to_hex());
                }

                Tag::new(tag)
            }
            Self::QuoteAddress {
                coordinate,
                relay_hint,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);
                tag.push(String::from(QUOTE));
                tag.push(coordinate.to_string());

                if let Some(relay_hint) = relay_hint {
                    tag.push(relay_hint.to_string());
                }

                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip18Tag);

fn parse_e_tag<T, S>(mut iter: T) -> Result<Nip18Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let id: S = iter.next().ok_or(Error::MissingEventId)?;
    let id: EventId = EventId::from_hex(id.as_ref())?;
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    Ok(Nip18Tag::Event { id, relay_hint })
}

fn parse_k_tag<T, S>(mut iter: T) -> Result<Nip18Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let kind: S = iter.next().ok_or(Error::MissingKind)?;
    let kind: u16 = kind.as_ref().parse().map_err(|_| Error::InvalidKind)?;
    Ok(Nip18Tag::Kind(Kind::from_u16(kind)))
}

fn parse_p_tag<T, S>(mut iter: T) -> Result<Nip18Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let public_key: S = iter.next().ok_or(Error::MissingPublicKey)?;
    let public_key: PublicKey = PublicKey::from_hex(public_key.as_ref())?;
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    Ok(Nip18Tag::PublicKey {
        public_key,
        relay_hint,
    })
}

fn parse_q_tag<T, S>(mut iter: T) -> Result<Nip18Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let value: S = iter.next().ok_or(Error::MissingEventId)?;
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    match EventId::from_hex(value.as_ref()) {
        Ok(id) => {
            let public_key: Option<PublicKey> = match iter.next() {
                Some(public_key) if !public_key.as_ref().is_empty() => {
                    Some(PublicKey::from_hex(public_key.as_ref())?)
                }
                _ => None,
            };

            Ok(Nip18Tag::Quote {
                id,
                relay_hint,
                public_key,
            })
        }
        Err(_) => Ok(Nip18Tag::QuoteAddress {
            coordinate: Coordinate::from_kpi_format(value.as_ref())?,
            relay_hint,
        }),
    }
}

#[cfg(all(test, feature = "std", feature = "os-rng"))]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_standardized_event_tag() {
        let relay_hint = RelayUrl::parse("wss://relay.example.com").unwrap();
        let tag = vec![
            String::from("e"),
            EventId::all_zeros().to_hex(),
            relay_hint.to_string(),
        ];
        let parsed = Nip18Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip18Tag::Event {
                id: EventId::all_zeros(),
                relay_hint: Some(relay_hint),
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_standardized_quote_tag() {
        let keys = Keys::generate();
        let relay_hint = RelayUrl::parse("wss://relay.example.com").unwrap();
        let tag = vec![
            String::from("q"),
            EventId::all_zeros().to_hex(),
            relay_hint.to_string(),
            keys.public_key().to_string(),
        ];
        let parsed = Nip18Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip18Tag::Quote {
                id: EventId::all_zeros(),
                relay_hint: Some(relay_hint),
                public_key: Some(keys.public_key()),
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_standardized_quote_address_tag() {
        let keys = Keys::generate();
        let coordinate =
            Coordinate::new(Kind::LongFormTextNote, keys.public_key()).identifier("article");
        let relay_hint = RelayUrl::parse("wss://relay.example.com").unwrap();
        let tag = vec![
            String::from("q"),
            coordinate.to_string(),
            relay_hint.to_string(),
        ];
        let parsed = Nip18Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip18Tag::QuoteAddress {
                coordinate,
                relay_hint: Some(relay_hint),
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }
}
