//! NIP-17: Private Direct Message
//!
//! <https://github.com/nostr-protocol/nips/blob/master/17.md>

use core::fmt;

use crate::types::url;
use crate::{Event, RelayUrl, Tag};

/// NIP-17 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Url error
    Url(url::Error),
    /// Missing tag kind
    MissingTagKind,
    /// Missing relay URL
    MissingRelayUrl,
    /// Unknown standardized tag
    UnknownStandardizedTag,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingRelayUrl => f.write_str("Missing relay URL"),
            Self::UnknownStandardizedTag => f.write_str("Unknown standardized tag"),
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
pub enum TagStandardNip17 {
    /// Relay
    ///
    /// `["relay", <relay URL>]`
    Relay(RelayUrl),
}

impl TagStandardNip17 {
    /// Parse NIP-17 standardized tag
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
            // Parse as "p" tag
            "p" => {
                let url: RelayUrl = parse_relay_tag(iter)?;
                Ok(Self::Relay(url))
            }
            _ => Err(Error::UnknownStandardizedTag),
        }
    }

    /// Serialize the standardized tag to a raw tag
    pub fn as_raw(&self) -> Tag {
        match self {
            Self::Relay(url) => {
                let mut tag: Vec<String> = Vec::with_capacity(2);

                tag.push(String::from("relay"));
                tag.push(url.to_string());

                Tag::new(tag)
            }
        }
    }
}

impl From<TagStandardNip17> for Tag {
    #[inline]
    fn from(standard: TagStandardNip17) -> Self {
        standard.as_raw()
    }
}

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
    event.tags.iter().filter_map(|tag| {
        if let Some(TagStandardNip17::Relay(url)) = TagStandardNip17::parse(tag.as_slice()).ok() {
            Some(url)
        } else {
            None
        }
    })
}
