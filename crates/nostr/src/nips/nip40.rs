//! NIP-40: Expiration Timestamp
//!
//! <https://github.com/nostr-protocol/nips/blob/master/40.md>

use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;

use crate::Tag;
use crate::types::time::Timestamp;

const EXPIRATION: &str = "expiration";

/// NIP-40 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Parse Int error
    ParseInt(ParseIntError),
    /// Missing tag kind
    MissingTagKind,
    /// Missing timestamp
    MissingTimestamp,
    /// Unknown standardized tag
    UnknownStandardizedTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseInt(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingTimestamp => f.write_str("Missing timestamp"),
            Self::UnknownStandardizedTag => f.write_str("Unknown standardized tag"),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}

/// Standardized NIP-40 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/40.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagStandardNip40 {
    /// Expiration timestamp
    Expiration(Timestamp),
}

impl TagStandardNip40 {
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
            EXPIRATION => {
                let timestamp: Timestamp = parse_expiration_tag(iter)?;
                Ok(Self::Expiration(timestamp))
            }
            _ => Err(Error::UnknownStandardizedTag),
        }
    }

    /// Serialize the standardized tag to a raw tag
    pub fn serialize(&self) -> Tag {
        match self {
            Self::Expiration(timestamp) => {
                let mut tag: Vec<String> = Vec::with_capacity(2);

                tag.push(String::from(EXPIRATION));
                tag.push(timestamp.to_string());

                Tag::new(tag)
            }
        }
    }
}

impl From<TagStandardNip40> for Tag {
    #[inline]
    fn from(standard: TagStandardNip40) -> Self {
        standard.serialize()
    }
}

fn parse_expiration_tag<T, S>(mut iter: T) -> Result<Timestamp, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    // Take and parse timestamp (index 1)
    let timestamp: S = iter.next().ok_or(Error::MissingTimestamp)?;
    let timestamp: Timestamp = Timestamp::from_str(timestamp.as_ref())?;

    Ok(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_tag() {
        let tag: Vec<String> = Vec::new();
        let err = TagStandardNip40::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingTagKind);
    }

    #[test]
    fn test_non_existing_tag() {
        let tag = vec!["hello"];
        let err = TagStandardNip40::parse(&tag).unwrap_err();
        assert_eq!(err, Error::UnknownStandardizedTag);
    }

    #[test]
    fn test_standardized_expiration_tag() {
        let raw = 1600000000;
        let timestamp = Timestamp::from_secs(raw);

        // Simple
        let tag = vec!["expiration".to_string(), raw.to_string()];
        let parsed = TagStandardNip40::parse(&tag).unwrap();
        assert_eq!(parsed, TagStandardNip40::Expiration(timestamp));
        assert_eq!(parsed.serialize(), Tag::parse(tag).unwrap());

        // Invalid timestamp
        let tag = vec!["expiration", "hello"];
        let err = TagStandardNip40::parse(&tag).unwrap_err();
        assert!(matches!(err, Error::ParseInt(_)));

        // Missing timestamp
        let tag = vec!["expiration"];
        let err = TagStandardNip40::parse(&tag).unwrap_err();
        assert_eq!(err, Error::MissingTimestamp);
    }
}
