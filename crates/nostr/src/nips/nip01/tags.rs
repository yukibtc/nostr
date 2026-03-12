use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::super::util::take_and_parse_relay_hint;
use super::{Coordinate, Error};
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::{EventId, PublicKey, RelayUrl};

/// Standardized NIP-01 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/01.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip01Tag {
    /// `a` tag
    Coordinate {
        /// Coordinate
        coordinate: Coordinate,
        /// Relay hint (optional but recommended)
        relay_hint: Option<RelayUrl>,
    },
    /// `e` tag
    Event {
        /// Event ID
        id: EventId,
        /// Relay hint (optional but recommended)
        relay_hint: Option<RelayUrl>,
        /// Public key hint
        public_key: Option<PublicKey>,
    },
    /// `d` tag
    Identifier(String),
    /// `p` tag
    PublicKey {
        /// Public key
        public_key: PublicKey,
        /// Relay hint (optional but recommended)
        relay_hint: Option<RelayUrl>,
    },
}

impl TagCodec for Nip01Tag {
    type Error = Error;

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
            // Parse as "a" tag
            "a" => {
                let (coordinate, relay_hint) = parse_a_tag(iter)?;
                Ok(Self::Coordinate {
                    coordinate,
                    relay_hint,
                })
            }
            // Parse as "e" tag
            "e" => {
                let (id, relay_hint, public_key) = parse_e_tag(iter)?;
                Ok(Self::Event {
                    id,
                    relay_hint,
                    public_key,
                })
            }
            "d" => {
                let identifier: S = iter.next().ok_or(Error::MissingIdentifier)?;
                Ok(Self::Identifier(identifier.as_ref().to_string()))
            }
            // Parse as "p" tag
            "p" => {
                let (public_key, relay_hint) = parse_p_tag(iter)?;
                Ok(Self::PublicKey {
                    public_key,
                    relay_hint,
                })
            }
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Coordinate {
                coordinate,
                relay_hint,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);

                tag.push(String::from("a"));
                tag.push(coordinate.to_string());

                if let Some(relay_hint) = relay_hint {
                    tag.push(relay_hint.to_string());
                }

                assert!(tag.len() >= 2);

                Tag::new(tag)
            }
            Self::Event {
                id,
                relay_hint,
                public_key,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(
                    2 + relay_hint.is_some() as usize + public_key.is_some() as usize,
                );

                tag.push(String::from("e"));
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
                    tag.push(public_key.to_string());
                }

                Tag::new(tag)
            }
            Self::Identifier(identifier) => {
                let tag: Vec<String> = vec![String::from("d"), identifier.to_string()];

                Tag::new(tag)
            }
            Self::PublicKey {
                public_key,
                relay_hint,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);

                tag.push(String::from("p"));
                tag.push(public_key.to_hex());

                if let Some(relay_hint) = relay_hint {
                    tag.push(relay_hint.to_string());
                }

                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip01Tag);

fn parse_a_tag<T, S>(mut iter: T) -> Result<(Coordinate, Option<RelayUrl>), Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse coordinate (index 1)
    let coordinate: S = iter.next().ok_or(Error::MissingCoordinate)?;
    let coordinate: Coordinate = Coordinate::from_kpi_format(coordinate.as_ref())?;

    // Take and parse relay hint (index 2)
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    Ok((coordinate, relay_hint))
}

fn parse_e_tag<T, S>(mut iter: T) -> Result<(EventId, Option<RelayUrl>, Option<PublicKey>), Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse event ID (index 1)
    let id: S = iter.next().ok_or(Error::MissingEventId)?;
    let id: EventId = EventId::from_hex(id.as_ref())?;

    // Take and parse relay hint (index 2)
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    // Take and parse public key (index 3)
    let public_key: Option<S> = iter.next();
    let public_key: Option<PublicKey> = match public_key {
        Some(pk) => {
            let pk: &str = pk.as_ref();

            if pk.is_empty() {
                None
            } else {
                Some(PublicKey::from_hex(pk)?)
            }
        }
        None => None,
    };

    Ok((id, relay_hint, public_key))
}

fn parse_p_tag<T, S>(mut iter: T) -> Result<(PublicKey, Option<RelayUrl>), Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse public key (index 1)
    let public_key: S = iter.next().ok_or(Error::MissingPublicKey)?;
    let public_key: PublicKey = PublicKey::from_hex(public_key.as_ref())?;

    // Take and parse relay hint (index 2)
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    Ok((public_key, relay_hint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, key};

    #[test]
    fn test_standardized_a_tag() {
        let raw = "30617:00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9:n34";
        let coordinate = Coordinate::from_kpi_format(raw).unwrap();

        // Simple
        let tag = vec!["a", raw];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Coordinate {
                coordinate: coordinate.clone(),
                relay_hint: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["a", raw, "wss://relay.damus.io/"];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Coordinate {
                coordinate,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap())
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // Invalid coordinate
        let tag = vec!["a", "hello"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::InvalidCoordinate);

        // Missing coordinate
        let tag = vec!["a"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingCoordinate);
    }

    #[test]
    fn test_standardized_e_tag() {
        let raw = "a3ce0a22c5c25e5a41a17004d38ed2aa8f815dda918c92400c6b611c41acbc78";
        let id = EventId::from_hex(raw).unwrap();

        // Simple
        let tag = vec!["e", raw];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Event {
                id,
                relay_hint: None,
                public_key: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["e", raw, "wss://relay.damus.io/"];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Event {
                id,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap()),
                public_key: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint and public key
        let tag = vec![
            "e",
            raw,
            "wss://relay.damus.io/",
            "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9",
        ];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Event {
                id,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap()),
                public_key: Some(
                    PublicKey::from_hex(
                        "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9"
                    )
                    .unwrap()
                )
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With public key and no relay hint
        let tag = vec![
            "e",
            raw,
            "",
            "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9",
        ];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::Event {
                id,
                relay_hint: None,
                public_key: Some(
                    PublicKey::from_hex(
                        "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9"
                    )
                    .unwrap()
                )
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // Invalid ID
        let tag = vec!["e", "hello"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert!(matches!(err, Error::Event(event::Error::Hex(_))));

        // Missing ID
        let tag = vec!["e"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingEventId);

        // Issue: https://gitworkshop.dev/yukikishimoto.com/nostr/issues/note15xl8ae8dnmt26adfw6ec8gshxxs242vrvsa3v36ctwq2x9gglkustlxlwa
        let result = Nip01Tag::parse(&["e", raw, "", "", ""]).unwrap();
        assert_eq!(
            result,
            Nip01Tag::Event {
                id,
                relay_hint: None,
                public_key: None,
            }
        )
    }

    #[test]
    fn test_standardized_d_tag() {
        let tag = vec!["d", "raw"];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(parsed, Nip01Tag::Identifier(String::from("raw")));
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // Missing identifier
        let tag = vec!["d"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingIdentifier);
    }

    #[test]
    fn test_standardized_p_tag() {
        let raw = "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9";
        let public_key = PublicKey::from_hex(raw).unwrap();

        // Simple
        let tag = vec!["p", raw];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::PublicKey {
                public_key,
                relay_hint: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["p", raw, "wss://relay.damus.io/"];
        let parsed = Nip01Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip01Tag::PublicKey {
                public_key,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap())
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // Invalid public key
        let tag = vec!["p", "hello"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert!(matches!(err, Error::Keys(key::Error::Hex(_))));

        // Missing public key
        let tag = vec!["p"];
        let err = Nip01Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingPublicKey);
    }
}
