// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP25: Reactions
//!
//! <https://github.com/nostr-protocol/nips/blob/master/25.md>

use alloc::string::{String, ToString};
use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;

use super::nip01::{Coordinate, Nip01Tag};
use crate::event::tag::{Tag, TagCodec, Tags, impl_tag_codec_conversions};
use crate::{Event, EventId, Kind, PublicKey, RelayUrl};

/// NIP-25 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Failed to parse integer
    ParseInt(ParseIntError),
    /// Missing kind
    MissingKind,
    /// Unknown tag
    UnknownTag,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseInt(e) => e.fmt(f),
            Self::MissingKind => f.write_str("Missing kind"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

/// Standardized NIP-25 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/25.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip25Tag {
    /// `k` tag
    Kind(Kind),
}

impl TagCodec for Nip25Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::UnknownTag)?;

        match kind.as_ref() {
            "k" => parse_k_tag(iter),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Kind(kind) => Tag::new(vec![String::from("k"), kind.to_string()]),
        }
    }
}

impl_tag_codec_conversions!(Nip25Tag);

fn parse_k_tag<T, S>(mut iter: T) -> Result<Nip25Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let kind: S = iter.next().ok_or(Error::MissingKind)?;

    let kind: Kind = Kind::from_str(kind.as_ref())?;
    Ok(Nip25Tag::Kind(kind))
}

/// Reaction target
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReactionTarget {
    /// Event ID
    pub event_id: EventId,
    /// Public Key
    pub public_key: PublicKey,
    /// Coordinate
    pub coordinate: Option<Coordinate>,
    /// Kind
    pub kind: Option<Kind>,
    /// Relay hint
    pub relay_hint: Option<RelayUrl>,
}

impl ReactionTarget {
    /// Construct a new reaction target
    pub fn new(event: &Event, relay_hint: Option<RelayUrl>) -> Self {
        Self {
            event_id: event.id,
            public_key: event.pubkey,
            coordinate: event.coordinate(),
            kind: Some(event.kind),
            relay_hint,
        }
    }

    pub(crate) fn into_tags(self) -> Tags {
        let mut tags: Tags = Tags::with_capacity(
            2 + usize::from(self.coordinate.is_some()) + usize::from(self.kind.is_some()),
        );

        // Serialization order: keep the `e` and `a` tags together, followed by the `p` and other tags.

        tags.push(
            Nip01Tag::Event {
                id: self.event_id,
                relay_hint: self.relay_hint.clone(),
                public_key: Some(self.public_key),
            }
            .to_tag(),
        );

        if let Some(coordinate) = self.coordinate {
            tags.push(
                Nip01Tag::Coordinate {
                    coordinate,
                    relay_hint: self.relay_hint.clone(),
                }
                .to_tag(),
            );
        }

        tags.push(
            Nip01Tag::PublicKey {
                public_key: self.public_key,
                relay_hint: self.relay_hint,
            }
            .to_tag(),
        );

        if let Some(kind) = self.kind {
            tags.push(Nip25Tag::Kind(kind).to_tag());
        }

        tags
    }
}

impl From<&Event> for ReactionTarget {
    fn from(event: &Event) -> Self {
        Self {
            event_id: event.id,
            public_key: event.pubkey,
            coordinate: event.coordinate(),
            kind: Some(event.kind),
            relay_hint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nip25_kind_tag() {
        let tag = vec!["k", "1"];
        let parsed = Nip25Tag::parse(&tag).unwrap();

        assert_eq!(parsed, Nip25Tag::Kind(Kind::TextNote));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_reaction_target_into_tags() {
        let event_id =
            EventId::from_hex("9ae37aa68f48645127299e9453eb5d908a0cbb6058ff340d528ed4d37c8994fb")
                .unwrap();
        let public_key =
            PublicKey::from_hex("04c915daefee38317fa734444acee390a8269fe5810b2241e5e6dd343dfbecc9")
                .unwrap();
        let relay_hint = RelayUrl::parse("wss://relay.example.com").unwrap();
        let coordinate = Coordinate::new(Kind::TextNote, public_key).identifier("reaction");

        let tags = ReactionTarget {
            event_id,
            public_key,
            coordinate: Some(coordinate.clone()),
            kind: Some(Kind::TextNote),
            relay_hint: Some(relay_hint.clone()),
        }
        .into_tags();

        assert_eq!(
            tags.first(),
            Some(
                &Nip01Tag::Event {
                    id: event_id,
                    relay_hint: Some(relay_hint.clone()),
                    public_key: Some(public_key),
                }
                .to_tag()
            )
        );
        assert_eq!(
            tags.get(1),
            Some(
                &Nip01Tag::Coordinate {
                    coordinate,
                    relay_hint: Some(relay_hint.clone()),
                }
                .to_tag()
            )
        );
        assert_eq!(
            tags.get(2),
            Some(
                &Nip01Tag::PublicKey {
                    public_key,
                    relay_hint: Some(relay_hint),
                }
                .to_tag()
            )
        );
        assert_eq!(tags.get(3), Some(&Nip25Tag::Kind(Kind::TextNote).to_tag()));
    }
}
