// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP-30: Custom Emoji
//!
//! <https://github.com/nostr-protocol/nips/blob/master/30.md>

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use super::nip01::{self, Coordinate};
use crate::Url;
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::types::url;

const EMOJI: &str = "emoji";

/// NIP-30 error
#[derive(Debug, PartialEq)]
pub enum Error {
    /// NIP-01 error
    Nip01(nip01::Error),
    /// URL parse error
    Url(url::ParseError),
    /// Missing tag kind
    MissingTagKind,
    /// Missing shortcode
    MissingShortcode,
    /// Missing image URL
    MissingImageUrl,
    /// Invalid shortcode
    InvalidShortcode,
    /// Unknown tag
    UnknownTag,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nip01(e) => e.fmt(f),
            Self::Url(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingShortcode => f.write_str("Missing shortcode"),
            Self::MissingImageUrl => f.write_str("Missing image URL"),
            Self::InvalidShortcode => f.write_str("Invalid shortcode"),
            Self::UnknownTag => f.write_str("Unknown tag"),
        }
    }
}

impl From<nip01::Error> for Error {
    fn from(e: nip01::Error) -> Self {
        Self::Nip01(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e)
    }
}

/// Standardized NIP-30 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/30.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip30Tag {
    /// `emoji` tag
    Emoji {
        /// Emoji shortcode
        shortcode: String,
        /// URL to image
        image_url: Url,
        /// Optional emoji set address
        emoji_set: Option<Coordinate>,
    },
}

impl TagCodec for Nip30Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            EMOJI => parse_emoji_tag(iter),
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Emoji {
                shortcode,
                image_url,
                emoji_set,
            } => {
                let mut tag: Vec<String> = Vec::with_capacity(3 + emoji_set.is_some() as usize);
                tag.push(String::from(EMOJI));
                tag.push(shortcode.clone());
                tag.push(image_url.to_string());

                if let Some(emoji_set) = emoji_set {
                    tag.push(emoji_set.to_string());
                }

                Tag::new(tag)
            }
        }
    }
}

impl_tag_codec_conversions!(Nip30Tag);

fn parse_emoji_tag<T, S>(mut iter: T) -> Result<Nip30Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let shortcode: S = iter.next().ok_or(Error::MissingShortcode)?;
    let shortcode: String = shortcode.as_ref().to_string();

    if !is_valid_shortcode(&shortcode) {
        return Err(Error::InvalidShortcode);
    }

    let image_url: S = iter.next().ok_or(Error::MissingImageUrl)?;
    let image_url: Url = Url::parse(image_url.as_ref())?;

    let emoji_set: Option<Coordinate> = match iter.next() {
        Some(emoji_set) => Some(Coordinate::from_kpi_format(emoji_set.as_ref())?),
        None => None,
    };

    Ok(Nip30Tag::Emoji {
        shortcode,
        image_url,
        emoji_set,
    })
}

fn is_valid_shortcode(shortcode: &str) -> bool {
    !shortcode.is_empty()
        && shortcode
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, PublicKey};

    #[test]
    fn test_nip30_emoji_tag() {
        let image_url = Url::parse("https://example.com/emoji.png").unwrap();
        let tag = vec!["emoji", "soapbox", "https://example.com/emoji.png"];
        let parsed = Nip30Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip30Tag::Emoji {
                shortcode: String::from("soapbox"),
                image_url,
                emoji_set: None,
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_nip30_emoji_tag_with_set() {
        let image_url = Url::parse("https://example.com/emoji.png").unwrap();
        let emoji_set = Coordinate::new(
            Kind::EmojiSet,
            PublicKey::from_hex("79c2cae114ea28a981e7559b4fe7854a473521a8d22a66bbab9fa248eb820ff6")
                .unwrap(),
        )
        .identifier("blobcats");
        let tag = vec![
            "emoji",
            "soapbox",
            "https://example.com/emoji.png",
            "30030:79c2cae114ea28a981e7559b4fe7854a473521a8d22a66bbab9fa248eb820ff6:blobcats",
        ];
        let parsed = Nip30Tag::parse(&tag).unwrap();

        assert_eq!(
            parsed,
            Nip30Tag::Emoji {
                shortcode: String::from("soapbox"),
                image_url,
                emoji_set: Some(emoji_set),
            }
        );
        assert_eq!(parsed.to_tag(), Tag::parse(tag).unwrap());
    }

    #[test]
    fn test_nip30_invalid_shortcode() {
        let tag = vec!["emoji", "soap box", "https://example.com/emoji.png"];
        assert_eq!(Nip30Tag::parse(&tag).unwrap_err(), Error::InvalidShortcode);
    }
}
