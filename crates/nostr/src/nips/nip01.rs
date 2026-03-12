//! NIP-01: Basic protocol flow description
//!
//! <https://github.com/nostr-protocol/nips/blob/master/01.md>

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::nip19::{self, FromBech32, Nip19Coordinate, ToBech32};
use super::nip21::{FromNostrUri, ToNostrUri};
use crate::types::url::{self, Url};
use crate::{EventId, Filter, JsonUtil, Kind, PublicKey, RelayUrl, Tag, event, key};

/// NIP-01 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Keys error
    Keys(key::Error),
    /// Event error
    Event(event::Error),
    /// Url error
    Url(url::Error),
    /// Parse Int error
    ParseInt(ParseIntError),
    /// Invalid coordinate
    InvalidCoordinate,
    /// Missing tag kind
    MissingTagKind,
    /// Missing event ID
    MissingEventId,
    /// Missing coordinate
    MissingCoordinate,
    /// Missing public key
    MissingPublicKey,
    /// Missing identifier
    MissingIdentifier,
    /// Unknown standardized tag
    UnknownStandardizedTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keys(e) => e.fmt(f),
            Self::Event(e) => e.fmt(f),
            Self::Url(e) => e.fmt(f),
            Self::ParseInt(e) => e.fmt(f),
            Self::InvalidCoordinate => f.write_str("Invalid coordinate"),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingEventId => f.write_str("Missing event ID"),
            Self::MissingCoordinate => f.write_str("Missing coordinate"),
            Self::MissingPublicKey => f.write_str("Missing public key"),
            Self::MissingIdentifier => f.write_str("Missing identifier"),
            Self::UnknownStandardizedTag => f.write_str("Unknown standardized tag"),
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

impl From<url::Error> for Error {
    fn from(e: url::Error) -> Self {
        Self::Url(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

/// Coordinate for event (`a` tag)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Coordinate {
    /// Kind
    pub kind: Kind,
    /// Public Key
    pub public_key: PublicKey,
    /// `d` tag identifier
    ///
    /// Needed for a parametrized replaceable event.
    /// Leave empty for a replaceable event.
    pub identifier: String,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.borrow().fmt(f)
    }
}

impl Coordinate {
    /// Create new event coordinate
    #[inline]
    pub fn new(kind: Kind, public_key: PublicKey) -> Self {
        Self {
            kind,
            public_key,
            identifier: String::new(),
        }
    }

    /// Parse coordinate from `<kind>:<pubkey>:[<d-tag>]` format, `bech32` or [NIP21](https://github.com/nostr-protocol/nips/blob/master/21.md) uri
    pub fn parse(coordinate: &str) -> Result<Self, Error> {
        // Try from hex
        if let Ok(coordinate) = Self::from_kpi_format(coordinate) {
            return Ok(coordinate);
        }

        // Try from bech32
        if let Ok(coordinate) = Self::from_bech32(coordinate) {
            return Ok(coordinate);
        }

        // Try from NIP21 URI
        if let Ok(coordinate) = Self::from_nostr_uri(coordinate) {
            return Ok(coordinate);
        }

        Err(Error::InvalidCoordinate)
    }

    /// Try to parse from `<kind>:<pubkey>:[<d-tag>]` format
    pub fn from_kpi_format(coordinate: &str) -> Result<Self, Error> {
        let mut kpi = coordinate.split(':');
        match (kpi.next(), kpi.next(), kpi.next()) {
            (Some(kind_str), Some(public_key_str), Some(identifier)) => Ok(Self {
                kind: Kind::from_str(kind_str)?,
                public_key: PublicKey::from_hex(public_key_str)?,
                identifier: identifier.to_string(),
            }),
            _ => Err(Error::InvalidCoordinate),
        }
    }

    /// Set a `d` tag identifier
    ///
    /// Needed for a parametrized replaceable event.
    pub fn identifier<S>(mut self, identifier: S) -> Self
    where
        S: Into<String>,
    {
        self.identifier = identifier.into();
        self
    }

    /// Check if coordinate has identifier
    #[inline]
    pub fn has_identifier(&self) -> bool {
        !self.identifier.is_empty()
    }

    /// Check if the coordinate is valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCoordinate`] if:
    /// - the [`Kind`] is `replaceable` and the identifier is not empty
    /// - the [`Kind`] is `addressable` and the identifier is empty
    #[inline]
    pub fn verify(&self) -> Result<(), Error> {
        verify_coordinate(&self.kind, &self.identifier)
    }

    /// Borrow coordinate
    pub fn borrow(&self) -> CoordinateBorrow<'_> {
        CoordinateBorrow {
            kind: &self.kind,
            public_key: &self.public_key,
            identifier: Some(&self.identifier),
        }
    }
}

fn verify_coordinate(kind: &Kind, identifier: &str) -> Result<(), Error> {
    let is_replaceable: bool = kind.is_replaceable();
    let is_addressable: bool = kind.is_addressable();

    if !is_replaceable && !is_addressable {
        return Err(Error::InvalidCoordinate);
    }

    if is_replaceable && !identifier.is_empty() {
        return Err(Error::InvalidCoordinate);
    }

    if is_addressable && identifier.is_empty() {
        return Err(Error::InvalidCoordinate);
    }

    Ok(())
}

impl From<Coordinate> for Tag {
    #[inline]
    fn from(coordinate: Coordinate) -> Self {
        Self::coordinate(coordinate)
    }
}

impl From<Coordinate> for Filter {
    fn from(value: Coordinate) -> Self {
        if value.identifier.is_empty() {
            Filter::new().kind(value.kind).author(value.public_key)
        } else {
            Filter::new()
                .kind(value.kind)
                .author(value.public_key)
                .identifier(value.identifier)
        }
    }
}

impl From<&Coordinate> for Filter {
    fn from(value: &Coordinate) -> Self {
        if value.identifier.is_empty() {
            Filter::new().kind(value.kind).author(value.public_key)
        } else {
            Filter::new()
                .kind(value.kind)
                .author(value.public_key)
                .identifier(value.identifier.clone())
        }
    }
}

impl FromStr for Coordinate {
    type Err = Error;

    /// Try to parse [Coordinate] from `<kind>:<pubkey>:[<d-tag>]` format, `bech32` or [NIP21](https://github.com/nostr-protocol/nips/blob/master/21.md) uri
    #[inline]
    fn from_str(coordinate: &str) -> Result<Self, Self::Err> {
        Self::parse(coordinate)
    }
}

impl ToBech32 for Coordinate {
    type Err = nip19::Error;

    #[inline]
    fn to_bech32(&self) -> Result<String, Self::Err> {
        self.borrow().to_bech32()
    }
}

impl FromBech32 for Coordinate {
    type Err = nip19::Error;

    fn from_bech32(addr: &str) -> Result<Self, Self::Err> {
        let coordinate: Nip19Coordinate = Nip19Coordinate::from_bech32(addr)?;
        Ok(coordinate.coordinate)
    }
}

impl ToNostrUri for Coordinate {}
impl FromNostrUri for Coordinate {}

/// Borrowed coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoordinateBorrow<'a> {
    /// Kind
    pub kind: &'a Kind,
    /// Public key
    pub public_key: &'a PublicKey,
    /// `d` tag identifier
    ///
    /// Needed for a parametrized replaceable event.
    pub identifier: Option<&'a str>,
}

impl CoordinateBorrow<'_> {
    /// Into owned coordinate
    pub fn into_owned(self) -> Coordinate {
        Coordinate {
            kind: *self.kind,
            public_key: *self.public_key,
            identifier: self.identifier.map(|s| s.to_string()).unwrap_or_default(),
        }
    }
}

impl fmt::Display for CoordinateBorrow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.kind,
            self.public_key,
            self.identifier.unwrap_or_default()
        )
    }
}

impl ToBech32 for CoordinateBorrow<'_> {
    type Err = nip19::Error;

    #[inline]
    fn to_bech32(&self) -> Result<String, Self::Err> {
        nip19::coordinate_to_bech32(*self, &[])
    }
}

impl ToNostrUri for CoordinateBorrow<'_> {}

/// Metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Metadata {
    /// Name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub display_name: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub about: Option<String>,
    /// Website url
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub website: Option<String>,
    /// Picture url
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub picture: Option<String>,
    /// Banner url
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub banner: Option<String>,
    /// NIP05 (ex. name@example.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub nip05: Option<String>,
    /// LNURL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub lud06: Option<String>,
    /// Lightning Address
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub lud16: Option<String>,
    /// Custom fields
    #[serde(
        flatten,
        serialize_with = "serialize_custom_fields",
        deserialize_with = "deserialize_custom_fields"
    )]
    #[serde(default)]
    pub custom: BTreeMap<String, Value>,
}

impl Metadata {
    /// New empty [`Metadata`]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name
    pub fn name<S>(self, name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    /// Set display name
    pub fn display_name<S>(self, display_name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            display_name: Some(display_name.into()),
            ..self
        }
    }

    /// Set about
    pub fn about<S>(self, about: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            about: Some(about.into()),
            ..self
        }
    }

    /// Set website
    pub fn website(self, url: Url) -> Self {
        Self {
            website: Some(url.into()),
            ..self
        }
    }

    /// Set picture
    pub fn picture(self, url: Url) -> Self {
        Self {
            picture: Some(url.into()),
            ..self
        }
    }

    /// Set banner
    pub fn banner(self, url: Url) -> Self {
        Self {
            banner: Some(url.into()),
            ..self
        }
    }

    /// Set nip05
    pub fn nip05<S>(self, nip05: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            nip05: Some(nip05.into()),
            ..self
        }
    }

    /// Set lud06 (LNURL)
    pub fn lud06<S>(self, lud06: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            lud06: Some(lud06.into()),
            ..self
        }
    }

    /// Set lud16 (Lightning Address)
    pub fn lud16<S>(self, lud16: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            lud16: Some(lud16.into()),
            ..self
        }
    }

    /// Set custom metadata field
    pub fn custom_field<K, S>(mut self, field_name: K, value: S) -> Self
    where
        K: Into<String>,
        S: Into<Value>,
    {
        self.custom.insert(field_name.into(), value.into());
        self
    }
}

impl JsonUtil for Metadata {
    type Err = serde_json::Error;
}

fn serialize_custom_fields<S>(
    custom_fields: &BTreeMap<String, Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(custom_fields.len()))?;
    for (field_name, value) in custom_fields {
        map.serialize_entry(field_name, value)?;
    }
    map.end()
}

fn deserialize_custom_fields<'de, D>(deserializer: D) -> Result<BTreeMap<String, Value>, D::Error>
where
    D: Deserializer<'de>,
{
    struct GenericTagsVisitor;

    impl<'de> Visitor<'de> for GenericTagsVisitor {
        type Value = BTreeMap<String, Value>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("map where keys are strings and values are valid json")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut custom_fields: BTreeMap<String, Value> = BTreeMap::new();
            while let Some(field_name) = map.next_key::<String>()? {
                if let Ok(value) = map.next_value::<Value>() {
                    custom_fields.insert(field_name, value);
                }
            }
            Ok(custom_fields)
        }
    }

    deserializer.deserialize_map(GenericTagsVisitor)
}

/// Standardized NIP-01 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/01.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagStandardNip01 {
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

impl TagStandardNip01 {
    pub fn parse<T, S>(tag: T) -> Result<Self, Error>
    where
        T: IntoIterator<Item = S>,
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
            _ => Err(Error::UnknownStandardizedTag),
        }
    }

    /// Serialize the standardized tag to a raw tag
    pub fn as_raw(&self) -> Tag {
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

                assert!(tag.len() >= 2);

                Tag::new(tag)
            }
            Self::Identifier(identifier) => {
                let mut tag: Vec<String> = Vec::with_capacity(2);

                tag.push(String::from("d"));
                tag.push(identifier.to_string());

                assert_eq!(tag.len(), 2);

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

                assert!(tag.len() >= 2);

                Tag::new(tag)
            }
        }
    }
}

impl From<TagStandardNip01> for Tag {
    #[inline]
    fn from(standard: TagStandardNip01) -> Self {
        standard.as_raw()
    }
}

fn take_and_parse_relay_hint<T, S>(iter: &mut T) -> Result<Option<RelayUrl>, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    match iter.next() {
        Some(url) => {
            let url: &str = url.as_ref();

            if url.is_empty() {
                Ok(None)
            } else {
                Ok(Some(RelayUrl::parse(url)?))
            }
        }
        None => Ok(None),
    }
}

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

    #[test]
    fn test_deserialize_metadata() {
        let content = r#"{"name":"myname","about":"Description","display_name":""}"#;
        let metadata = Metadata::from_json(content).unwrap();
        assert_eq!(
            metadata,
            Metadata::new()
                .name("myname")
                .about("Description")
                .display_name("")
        );

        let content = r#"{"name":"myname","about":"Description","displayName":"Jack"}"#;
        let metadata = Metadata::from_json(content).unwrap();
        assert_eq!(
            metadata,
            Metadata::new()
                .name("myname")
                .about("Description")
                .custom_field("displayName", "Jack")
        );

        let content = r#"{"lud16":"thesimplekid@cln.thesimplekid.com","nip05":"_@thesimplekid.com","display_name":"thesimplekid","about":"Wannabe open source dev","name":"thesimplekid","username":"thesimplekid","displayName":"thesimplekid","lud06":"","reactions":false,"damus_donation_v2":0}"#;
        let metadata = Metadata::from_json(content).unwrap();
        assert_eq!(
            metadata,
            Metadata::new()
                .name("thesimplekid")
                .display_name("thesimplekid")
                .about("Wannabe open source dev")
                .nip05("_@thesimplekid.com")
                .lud06("")
                .lud16("thesimplekid@cln.thesimplekid.com")
                .custom_field("username", "thesimplekid")
                .custom_field("displayName", "thesimplekid")
                .custom_field("reactions", false)
                .custom_field("damus_donation_v2", 0)
        );
        assert_eq!(metadata, Metadata::from_json(metadata.as_json()).unwrap());
    }

    #[test]
    fn parse_valid_coordinate() {
        let coordinate: &str =
            "30023:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:ipsum";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();

        let expected_public_key: PublicKey =
            PublicKey::from_hex("aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4")
                .unwrap();

        assert_eq!(coordinate.kind.as_u16(), 30023);
        assert_eq!(coordinate.public_key, expected_public_key);
        assert_eq!(coordinate.identifier, "ipsum");

        let coordinate: &str =
            "20500:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();

        assert_eq!(coordinate.kind.as_u16(), 20500);
        assert_eq!(coordinate.public_key, expected_public_key);
        assert_eq!(coordinate.identifier, "");
    }

    #[test]
    fn test_verify_coordinate() {
        // Valid: replaceable
        let coordinate: &str =
            "15000:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();
        assert!(coordinate.verify().is_ok());

        // Valid: addressable
        let coordinate: &str =
            "30023:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:ipsum";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();
        assert!(coordinate.verify().is_ok());

        // Invalid: ephemeral kind
        let coordinate: &str =
            "20500:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();
        assert!(coordinate.verify().is_err());

        // Invalid: replaceable with identifier
        let coordinate: &str =
            "11111:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:test";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();
        assert!(coordinate.verify().is_err());

        // Invalid: addressable without identifier
        let coordinate: &str =
            "30023:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:";
        let coordinate: Coordinate = Coordinate::parse(coordinate).unwrap();
        assert!(coordinate.verify().is_err());
    }

    #[test]
    fn display_addressable_coordinate() {
        let pkey = "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9";
        let coordinate = Coordinate::new(
            Kind::GitRepoAnnouncement,
            PublicKey::from_hex(pkey).unwrap(),
        )
        .identifier("n34");

        assert_eq!(
            coordinate.to_string(),
            "30617:00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9:n34"
        )
    }

    #[test]
    fn display_replaceable_coordinate() {
        let pkey = "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9";
        let coordinate = Coordinate::new(Kind::MuteList, PublicKey::from_hex(pkey).unwrap());

        assert_eq!(
            coordinate.to_string(),
            "10000:00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9:"
        )
    }

    #[test]
    fn test_standardized_a_tag() {
        let raw = "30617:00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9:n34";
        let coordinate = Coordinate::from_kpi_format(raw).unwrap();

        // Simple
        let tag = vec!["a", raw];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Coordinate {
                coordinate: coordinate.clone(),
                relay_hint: None
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["a", raw, "wss://relay.damus.io/"];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Coordinate {
                coordinate,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap())
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // Invalid coordinate
        let tag = vec!["a", "hello"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::InvalidCoordinate);

        // Missing coordinate
        let tag = vec!["a"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingCoordinate);
    }

    #[test]
    fn test_standardized_e_tag() {
        let raw = "a3ce0a22c5c25e5a41a17004d38ed2aa8f815dda918c92400c6b611c41acbc78";
        let id = EventId::from_hex(raw).unwrap();

        // Simple
        let tag = vec!["e", raw];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Event {
                id,
                relay_hint: None,
                public_key: None
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["e", raw, "wss://relay.damus.io/"];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Event {
                id,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap()),
                public_key: None
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // With relay hint and public key
        let tag = vec![
            "e",
            raw,
            "wss://relay.damus.io/",
            "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9",
        ];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Event {
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
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // With public key and no relay hint
        let tag = vec![
            "e",
            raw,
            "",
            "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9",
        ];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::Event {
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
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // Invalid ID
        let tag = vec!["e", "hello"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::Event(event::Error::InvalidId));

        // Missing ID
        let tag = vec!["e"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingEventId);

        // Issue: https://gitworkshop.dev/yukikishimoto.com/nostr/issues/note15xl8ae8dnmt26adfw6ec8gshxxs242vrvsa3v36ctwq2x9gglkustlxlwa
        let result = TagStandardNip01::parse(&["e", raw, "", "", ""]).unwrap();
        assert_eq!(
            result,
            TagStandardNip01::Event {
                id,
                relay_hint: None,
                public_key: None,
            }
        )
    }

    #[test]
    fn test_standardized_d_tag() {
        let tag = vec!["d", "raw"];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(parsed, TagStandardNip01::Identifier(String::from("raw")));
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // Missing identifier
        let tag = vec!["d"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingIdentifier);
    }

    #[test]
    fn test_standardized_p_tag() {
        let raw = "00000001505e7e48927046e9bbaa728b1f3b511227e2200c578d6e6bb0c77eb9";
        let public_key = PublicKey::from_hex(raw).unwrap();

        // Simple
        let tag = vec!["p", raw];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::PublicKey {
                public_key,
                relay_hint: None
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // With relay hint
        let tag = vec!["p", raw, "wss://relay.damus.io/"];
        let parsed = TagStandardNip01::parse(&tag).unwrap();
        assert_eq!(
            parsed,
            TagStandardNip01::PublicKey {
                public_key,
                relay_hint: Some(RelayUrl::parse("wss://relay.damus.io/").unwrap())
            }
        );
        assert_eq!(parsed.as_raw(), Tag::parse(tag).unwrap());

        // Invalid public key
        let tag = vec!["p", "hello"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::Keys(key::Error::InvalidPublicKey));

        // Missing public key
        let tag = vec!["p"];
        let err = TagStandardNip01::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingPublicKey);
    }
}

#[cfg(bench)]
mod benches {
    use test::{Bencher, black_box};

    use super::*;

    #[bench]
    pub fn parse_coordinate(bh: &mut Bencher) {
        let coordinate: &str =
            "30023:aa4fc8665f5696e33db7e1a572e3b0f5b3d615837b0f362dcb1c8068b098c7b4:ipsum";
        bh.iter(|| {
            black_box(Coordinate::parse(coordinate)).unwrap();
        });
    }
}
