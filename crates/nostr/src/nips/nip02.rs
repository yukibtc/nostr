// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-02: Follow List
//!
//! <https://github.com/nostr-protocol/nips/blob/master/02.md>

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use super::util::take_and_parse_relay_hint;
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::key::{self, PublicKey};
use crate::types::url::{self, RelayUrl};

/// NIP-02 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Keys error
    Keys(key::Error),
    /// Url error
    Url(url::Error),
    /// Missing tag kind
    MissingTagKind,
    /// Missing public key
    MissingPublicKey,
    /// Unknown tag
    UnknownTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keys(e) => e.fmt(f),
            Self::Url(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingPublicKey => f.write_str("Missing public key"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

impl From<key::Error> for Error {
    fn from(e: key::Error) -> Self {
        Self::Keys(e)
    }
}

impl From<url::Error> for Error {
    fn from(e: url::Error) -> Self {
        Self::Url(e)
    }
}

/// Contact
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Contact {
    /// Public key
    pub public_key: PublicKey,
    /// Relay url
    pub relay_url: Option<RelayUrl>,
    /// Alias
    pub alias: Option<String>,
}

impl Contact {
    /// Create new contact
    #[inline]
    pub fn new(public_key: PublicKey) -> Self {
        Self {
            public_key,
            relay_url: None,
            alias: None,
        }
    }
}

impl From<Contact> for Nip02Tag {
    fn from(contact: Contact) -> Self {
        Self::PublicKey {
            public_key: contact.public_key,
            relay_hint: contact.relay_url,
            alias: contact.alias,
        }
    }
}

/// Standardized NIP-02 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/02.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip02Tag {
    /// Contact public key
    ///
    /// `["p", <32-bytes hex key>, <main relay URL>, <petname>]`
    PublicKey {
        /// Public key
        public_key: PublicKey,
        /// Recommended relay URL
        relay_hint: Option<RelayUrl>,
        /// Alias
        alias: Option<String>,
    },
}

impl TagCodec for Nip02Tag {
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
            // Parse as "p" tag
            "p" => {
                let (public_key, relay_hint, alias) = parse_p_tag(iter)?;
                Ok(Self::PublicKey {
                    public_key,
                    relay_hint,
                    alias,
                })
            }
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::PublicKey {
                public_key,
                relay_hint,
                alias,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(2 + relay_hint.is_some() as usize);

                tag.push(String::from("p"));
                tag.push(public_key.to_hex());

                match relay_hint {
                    Some(relay_hint) => tag.push(relay_hint.to_string()),
                    None => {
                        if alias.is_some() {
                            tag.push(String::new());
                        }
                    }
                }

                if let Some(alias) = alias {
                    tag.push(alias.to_string());
                }

                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip02Tag);

fn parse_p_tag<T, S>(mut iter: T) -> Result<(PublicKey, Option<RelayUrl>, Option<String>), Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse public key (index 1)
    let public_key: S = iter.next().ok_or(Error::MissingPublicKey)?;
    let public_key: PublicKey = PublicKey::from_hex(public_key.as_ref())?;

    // Take and parse relay hint (index 2)
    let relay_hint: Option<RelayUrl> = take_and_parse_relay_hint(&mut iter)?;

    // Take and parse alias (index 3)
    let alias: Option<String> = match iter.next() {
        Some(alias) => {
            let alias: &str = alias.as_ref();

            if alias.is_empty() {
                None
            } else {
                Some(alias.to_string())
            }
        }
        None => None,
    };

    Ok((public_key, relay_hint, alias))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standardized_p_tag() {
        let raw = "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9";
        let public_key = PublicKey::from_hex(raw).unwrap();

        // Simple
        let tag = vec!["p", raw];
        let parsed = Nip02Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip02Tag::PublicKey {
                public_key,
                relay_hint: None,
                alias: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["p", raw, "wss://relay.damus.io/"];
        let parsed = Nip02Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip02Tag::PublicKey {
                public_key,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap()),
                alias: None
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With relay hint and alias
        let tag = vec!["p", raw, "wss://relay.damus.io/", "alice"];
        let parsed = Nip02Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip02Tag::PublicKey {
                public_key,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap()),
                alias: Some(String::from("alice"))
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // With alias and no relay hint
        let tag = vec!["p", raw, "", "alice"];
        let parsed = Nip02Tag::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            Nip02Tag::PublicKey {
                public_key,
                relay_hint: None,
                alias: Some(String::from("alice"))
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());

        // Invalid public key
        let tag = vec!["p", "hello"];
        let err = Nip02Tag::parse(&tag).unwrap_err();
        assert!(matches!(err, Error::Keys(key::Error::Hex(_))));

        // Missing public key
        let tag = vec!["p"];
        let err = Nip02Tag::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingPublicKey);
    }
}
