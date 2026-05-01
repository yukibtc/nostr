// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! NIP57: Lightning Zaps
//!
//! <https://github.com/nostr-protocol/nips/blob/master/57.md>

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::num::ParseIntError;

use aes::Aes256;
#[cfg(feature = "rand")]
use aes::cipher::BlockEncryptMut;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
#[cfg(feature = "rand")]
use bech32::Bech32;
use bech32::Hrp;
use cbc::Decryptor;
#[cfg(feature = "rand")]
use cbc::Encryptor;
use hashes::Hash;
use hashes::sha256::Hash as Sha256Hash;
#[cfg(all(feature = "std", feature = "os-rng"))]
use rand::TryRngCore;
#[cfg(all(feature = "std", feature = "os-rng"))]
use rand::rngs::OsRng;
#[cfg(feature = "rand")]
use rand::{CryptoRng, RngCore};
#[cfg(feature = "rand")]
use secp256k1::{Secp256k1, Signing, Verification};

use super::nip01::Coordinate;
#[cfg(all(feature = "std", feature = "os-rng"))]
use crate::SECP256K1;
use crate::event::builder::Error as BuilderError;
use crate::event::tag::{Tag, TagCodec, impl_tag_codec_conversions};
use crate::key::Error as KeyError;
use crate::types::url;
use crate::{Event, EventId, JsonUtil, PublicKey, RelayUrl, SecretKey, Timestamp, event, util};
#[cfg(feature = "rand")]
use crate::{EventBuilder, Keys, Kind};

#[cfg(feature = "rand")]
type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

const PRIVATE_ZAP_MSG_BECH32_PREFIX: Hrp = Hrp::parse_unchecked("pzap");
const PRIVATE_ZAP_IV_BECH32_PREFIX: Hrp = Hrp::parse_unchecked("iv");
const ANON: &str = "anon";
const AMOUNT: &str = "amount";
const BOLT11: &str = "bolt11";
const DESCRIPTION: &str = "description";
const LNURL: &str = "lnurl";
const PREIMAGE: &str = "preimage";
const RELAYS: &str = "relays";

#[allow(missing_docs)]
#[derive(Debug)]
pub enum Error {
    Key(KeyError),
    Builder(BuilderError),
    Event(event::Error),
    Url(url::Error),
    ParseInt(ParseIntError),
    Bech32Decode(bech32::DecodeError),
    Bech32Encode(bech32::EncodeError),
    MissingTagKind,
    MissingAmount,
    MissingBolt11,
    MissingDescription,
    MissingLnurl,
    MissingPreimage,
    InvalidPrivateZapMessage,
    PrivateZapMessageNotFound,
    UnknownTag,
    /// Wrong prefix or variant
    WrongBech32Prefix,
    /// Wrong encryption block mode
    WrongBlockMode,
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(e) => e.fmt(f),
            Self::Builder(e) => e.fmt(f),
            Self::Event(e) => e.fmt(f),
            Self::Url(e) => e.fmt(f),
            Self::ParseInt(e) => e.fmt(f),
            Self::Bech32Decode(e) => e.fmt(f),
            Self::Bech32Encode(e) => e.fmt(f),
            Self::MissingTagKind => f.write_str("Missing tag kind"),
            Self::MissingAmount => f.write_str("Missing amount"),
            Self::MissingBolt11 => f.write_str("Missing bolt11"),
            Self::MissingDescription => f.write_str("Missing description"),
            Self::MissingLnurl => f.write_str("Missing lnurl"),
            Self::MissingPreimage => f.write_str("Missing preimage"),
            Self::InvalidPrivateZapMessage => f.write_str("Invalid private zap message"),
            Self::PrivateZapMessageNotFound => f.write_str("Private zap message not found"),
            Self::UnknownTag => f.write_str("Unknown tag"),
            Self::WrongBech32Prefix => f.write_str("Wrong bech32 prefix"),
            Self::WrongBlockMode => f.write_str(
                "Wrong encryption block mode. The content must be encrypted using CBC mode!",
            ),
        }
    }
}

impl From<KeyError> for Error {
    fn from(e: KeyError) -> Self {
        Self::Key(e)
    }
}

impl From<BuilderError> for Error {
    fn from(e: BuilderError) -> Self {
        Self::Builder(e)
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

impl From<bech32::DecodeError> for Error {
    fn from(e: bech32::DecodeError) -> Self {
        Self::Bech32Decode(e)
    }
}

/// Standardized NIP-57 tags
///
/// <https://github.com/nostr-protocol/nips/blob/master/57.md>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nip57Tag {
    /// `relays` tag
    Relays(Vec<RelayUrl>),
    /// `amount` tag
    Amount {
        /// Amount in millisats
        millisats: u64,
        /// Optional bolt11 invoice
        bolt11: Option<String>,
    },
    /// `lnurl` tag
    Lnurl(String),
    /// `anon` tag
    Anon {
        /// Optional private zap payload
        msg: Option<String>,
    },
    /// `bolt11` tag
    Bolt11(String),
    /// `description` tag
    Description(String),
    /// `preimage` tag
    Preimage(String),
}

impl TagCodec for Nip57Tag {
    type Error = Error;

    fn parse<I, S>(tag: I) -> Result<Self, Self::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut iter = tag.into_iter();
        let kind: S = iter.next().ok_or(Error::MissingTagKind)?;

        match kind.as_ref() {
            RELAYS => Ok(Self::Relays(parse_relays(iter)?)),
            AMOUNT => parse_amount_tag(iter),
            LNURL => {
                let lnurl: S = iter.next().ok_or(Error::MissingLnurl)?;
                Ok(Self::Lnurl(lnurl.as_ref().to_string()))
            }
            ANON => {
                let msg: Option<String> = match iter.next() {
                    Some(msg) if !msg.as_ref().is_empty() => Some(msg.as_ref().to_string()),
                    _ => None,
                };
                Ok(Self::Anon { msg })
            }
            BOLT11 => {
                let bolt11: S = iter.next().ok_or(Error::MissingBolt11)?;
                Ok(Self::Bolt11(bolt11.as_ref().to_string()))
            }
            DESCRIPTION => {
                let description: S = iter.next().ok_or(Error::MissingDescription)?;
                Ok(Self::Description(description.as_ref().to_string()))
            }
            PREIMAGE => {
                let preimage: S = iter.next().ok_or(Error::MissingPreimage)?;
                Ok(Self::Preimage(preimage.as_ref().to_string()))
            }
            _ => Err(Error::UnknownTag),
        }
    }

    fn to_tag(&self) -> Tag {
        match self {
            Self::Relays(relays) => {
                let mut tag: Vec<String> = Vec::with_capacity(relays.len() + 1);
                tag.push(String::from(RELAYS));
                tag.extend(relays.iter().map(ToString::to_string));
                Tag::new(tag)
            }
            Self::Amount { millisats, bolt11 } => {
                let mut tag: Vec<String> = vec![String::from(AMOUNT), millisats.to_string()];
                if let Some(bolt11) = bolt11 {
                    tag.push(bolt11.clone());
                }
                Tag::new(tag)
            }
            Self::Lnurl(lnurl) => Tag::new(vec![String::from(LNURL), lnurl.clone()]),
            Self::Anon { msg } => {
                let mut tag: Vec<String> = vec![String::from(ANON)];
                if let Some(msg) = msg {
                    tag.push(msg.clone());
                }
                Tag::new(tag)
            }
            Self::Bolt11(bolt11) => Tag::new(vec![String::from(BOLT11), bolt11.clone()]),
            Self::Description(description) => {
                Tag::new(vec![String::from(DESCRIPTION), description.clone()])
            }
            Self::Preimage(preimage) => Tag::new(vec![String::from(PREIMAGE), preimage.clone()]),
        }
    }
}

impl_tag_codec_conversions!(Nip57Tag);

fn parse_relays<T, S>(iter: T) -> Result<Vec<RelayUrl>, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut relays: Vec<RelayUrl> = Vec::new();

    for relay in iter {
        relays.push(RelayUrl::parse(relay.as_ref())?);
    }

    Ok(relays)
}

fn parse_amount_tag<T, S>(mut iter: T) -> Result<Nip57Tag, Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    let millisats: S = iter.next().ok_or(Error::MissingAmount)?;
    let millisats: u64 = millisats.as_ref().parse()?;
    let bolt11: Option<String> = iter.next().map(|bolt11| bolt11.as_ref().to_string());

    Ok(Nip57Tag::Amount { millisats, bolt11 })
}

impl From<bech32::EncodeError> for Error {
    fn from(e: bech32::EncodeError) -> Self {
        Self::Bech32Encode(e)
    }
}

/// Zap Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ZapType {
    /// Public
    Public,
    /// Private
    Private,
    /// Anonymous
    Anonymous,
}

/// Zap Request Data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZapRequestData {
    /// Public key of the recipient
    pub public_key: PublicKey,
    /// List of relays the recipient's wallet should publish its zap receipt to
    pub relays: Vec<RelayUrl>,
    /// Message
    pub message: String,
    /// Amount in `millisats` the sender intends to pay
    pub amount: Option<u64>,
    /// Lnurl pay url of the recipient, encoded using bech32 with the prefix lnurl.
    pub lnurl: Option<String>,
    /// Event ID
    pub event_id: Option<EventId>,
    /// NIP33 event coordinate that allows tipping parameterized replaceable events such as NIP23 long-form notes.
    pub event_coordinate: Option<Coordinate>,
}

impl ZapRequestData {
    /// New Zap Request Data
    pub fn new<I>(public_key: PublicKey, relays: I) -> Self
    where
        I: IntoIterator<Item = RelayUrl>,
    {
        Self {
            public_key,
            relays: relays.into_iter().collect(),
            message: String::new(),
            amount: None,
            lnurl: None,
            event_id: None,
            event_coordinate: None,
        }
    }

    /// Message
    pub fn message<S>(self, message: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            message: message.into(),
            ..self
        }
    }

    /// Amount in `millisats` the sender intends to pay
    pub fn amount(self, amount: u64) -> Self {
        Self {
            amount: Some(amount),
            ..self
        }
    }

    /// Lnurl pay url of the recipient, encoded using bech32 with the prefix lnurl.
    pub fn lnurl<S>(self, lnurl: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            lnurl: Some(lnurl.into()),
            ..self
        }
    }

    /// Event ID
    pub fn event_id(self, event_id: EventId) -> Self {
        Self {
            event_id: Some(event_id),
            ..self
        }
    }

    /// NIP33 event coordinate that allows tipping parameterized replaceable events such as NIP23 long-form notes.
    pub fn event_coordinate(self, event_coordinate: Coordinate) -> Self {
        Self {
            event_coordinate: Some(event_coordinate),
            ..self
        }
    }
}

impl From<ZapRequestData> for Vec<Tag> {
    fn from(data: ZapRequestData) -> Self {
        let ZapRequestData {
            public_key,
            relays,
            amount,
            lnurl,
            event_id,
            event_coordinate,
            ..
        } = data;

        let mut tags: Vec<Tag> = vec![Tag::public_key(public_key)];

        if !relays.is_empty() {
            tags.push(Nip57Tag::Relays(relays).into());
        }

        if let Some(event_id) = event_id {
            tags.push(Tag::event(event_id));
        }

        if let Some(event_coordinate) = event_coordinate {
            tags.push(event_coordinate.into());
        }

        if let Some(amount) = amount {
            tags.push(
                Nip57Tag::Amount {
                    millisats: amount,
                    bolt11: None,
                }
                .into(),
            );
        }

        if let Some(lnurl) = lnurl {
            tags.push(Nip57Tag::Lnurl(lnurl).into());
        }

        tags
    }
}

/// Create **anonymous** zap request
#[cfg(all(feature = "std", feature = "os-rng"))]
pub fn anonymous_zap_request(data: ZapRequestData) -> Result<Event, Error> {
    let keys = Keys::generate();
    let message: String = data.message.clone();
    let mut tags: Vec<Tag> = data.into();
    tags.push(Nip57Tag::Anon { msg: None }.into());
    Ok(EventBuilder::new(Kind::ZapRequest, message)
        .tags(tags)
        .sign_with_keys(&keys)?)
}

/// Create **private** zap request
#[inline]
#[cfg(all(feature = "std", feature = "os-rng"))]
pub fn private_zap_request(data: ZapRequestData, keys: &Keys) -> Result<Event, Error> {
    private_zap_request_with_ctx(&SECP256K1, &mut OsRng.unwrap_err(), data, keys)
}

/// Create **private** zap request
#[cfg(feature = "rand")]
pub fn private_zap_request_with_ctx<C, R>(
    secp: &Secp256k1<C>,
    rng: &mut R,
    data: ZapRequestData,
    keys: &Keys,
) -> Result<Event, Error>
where
    C: Signing + Verification,
    R: RngCore + CryptoRng,
{
    let created_at: Timestamp = Timestamp::now();

    // Create encryption key
    let secret_key: SecretKey =
        create_encryption_key(keys.secret_key(), &data.public_key, created_at)?;

    // Compose encrypted message
    let mut tags: Vec<Tag> = vec![Tag::public_key(data.public_key)];
    if let Some(event_id) = data.event_id {
        tags.push(Tag::event(event_id));
    }
    let msg: String = EventBuilder::new(Kind::ZapPrivateMessage, &data.message)
        .tags(tags)
        .sign_with_ctx(secp, rng, keys)?
        .as_json();
    let msg: String = encrypt_private_zap_message(rng, &secret_key, &data.public_key, msg)?;

    // Compose event
    let mut tags: Vec<Tag> = data.into();
    tags.push(Nip57Tag::Anon { msg: Some(msg) }.into());
    let private_zap_keys: Keys = Keys::new_with_ctx(secp, secret_key);
    Ok(EventBuilder::new(Kind::ZapRequest, "")
        .tags(tags)
        .custom_created_at(created_at)
        .sign_with_ctx(secp, rng, &private_zap_keys)?)
}

/// Create NIP57 encryption key for **private** zap
pub fn create_encryption_key(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    created_at: Timestamp,
) -> Result<SecretKey, Error> {
    let mut unhashed: String = secret_key.to_secret_hex();
    unhashed.push_str(&public_key.to_string());
    unhashed.push_str(&created_at.to_string());
    let hash = Sha256Hash::hash(unhashed.as_bytes());
    Ok(SecretKey::from_slice(hash.as_byte_array())?)
}

/// Encrypt a private zap message using the given keys
#[cfg(feature = "rand")]
pub fn encrypt_private_zap_message<R, T>(
    rng: &mut R,
    secret_key: &SecretKey,
    public_key: &PublicKey,
    msg: T,
) -> Result<String, Error>
where
    R: RngCore,
    T: AsRef<[u8]>,
{
    let key: [u8; 32] = util::generate_shared_key(secret_key, public_key)?;
    let mut iv: [u8; 16] = [0u8; 16];
    rng.fill_bytes(&mut iv);

    let cipher = Aes256CbcEnc::new(&key.into(), &iv.into());
    let msg: Vec<u8> = cipher.encrypt_padded_vec_mut::<Pkcs7>(msg.as_ref());

    // Bech32 msg
    let encrypted_bech32_msg: String =
        bech32::encode::<Bech32>(PRIVATE_ZAP_MSG_BECH32_PREFIX, &msg)?;

    // Bech32 IV
    let iv_bech32: String = bech32::encode::<Bech32>(PRIVATE_ZAP_IV_BECH32_PREFIX, &iv)?;

    Ok(format!("{encrypted_bech32_msg}_{iv_bech32}"))
}

fn extract_anon_tag_message(event: &Event) -> Result<String, Error> {
    for tag in event.tags.iter() {
        if let Ok(Nip57Tag::Anon { msg }) = Nip57Tag::try_from(tag) {
            return msg.ok_or(Error::InvalidPrivateZapMessage);
        }
    }
    Err(Error::PrivateZapMessageNotFound)
}

/// Decrypt **private** zap message that was sent by the owner of the secret key
pub fn decrypt_sent_private_zap_message(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    private_zap_event: &Event,
) -> Result<Event, Error> {
    // Re-create our ephemeral encryption key
    let secret_key: SecretKey =
        create_encryption_key(secret_key, public_key, private_zap_event.created_at)?;
    let key: [u8; 32] = util::generate_shared_key(&secret_key, public_key)?;

    // decrypt like normal
    decrypt_private_zap_message(key, private_zap_event)
}

/// Decrypt **private** zap message that was received by the owner of the secret key
#[inline]
pub fn decrypt_received_private_zap_message(
    secret_key: &SecretKey,
    private_zap_event: &Event,
) -> Result<Event, Error> {
    let key: [u8; 32] = util::generate_shared_key(secret_key, &private_zap_event.pubkey)?;
    decrypt_private_zap_message(key, private_zap_event)
}

fn decrypt_private_zap_message(key: [u8; 32], private_zap_event: &Event) -> Result<Event, Error> {
    let msg: String = extract_anon_tag_message(private_zap_event)?;
    let mut splitted = msg.split('_');

    let msg: &str = splitted.next().ok_or(Error::InvalidPrivateZapMessage)?;
    let iv: &str = splitted.next().ok_or(Error::InvalidPrivateZapMessage)?;

    // IV
    let (hrp, iv) = bech32::decode(iv)?;
    if hrp != PRIVATE_ZAP_IV_BECH32_PREFIX {
        return Err(Error::WrongBech32Prefix);
    }

    // Msg
    let (hrp, msg) = bech32::decode(msg)?;
    if hrp != PRIVATE_ZAP_MSG_BECH32_PREFIX {
        return Err(Error::WrongBech32Prefix);
    }

    // Decrypt
    let cipher = Aes256CbcDec::new(&key.into(), iv.as_slice().into());
    let result: Vec<u8> = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(&msg)
        .map_err(|_| Error::WrongBlockMode)?;

    // TODO: check if event kind is equal to 9733
    Ok(Event::from_json(result)?)
}

#[cfg(test)]
#[cfg(all(feature = "std", feature = "os-rng"))]
mod tests {
    use super::*;

    #[test]
    fn test_nip57_relays_tag() {
        let tag = vec!["relays", "wss://relay.damus.io", "wss://relay.primal.net"];
        let parsed = Nip57Tag::parse(tag).unwrap();

        assert_eq!(
            parsed,
            Nip57Tag::Relays(vec![
                RelayUrl::parse("wss://relay.damus.io").unwrap(),
                RelayUrl::parse("wss://relay.primal.net").unwrap(),
            ])
        );
        assert_eq!(
            parsed.to_tag(),
            Tag::parse(["relays", "wss://relay.damus.io", "wss://relay.primal.net"]).unwrap()
        );
    }

    #[test]
    fn test_nip57_amount_tag() {
        let tag = vec!["amount", "21000", "lnbc21u1p0test"];
        let parsed = Nip57Tag::parse(tag).unwrap();

        assert_eq!(
            parsed,
            Nip57Tag::Amount {
                millisats: 21000,
                bolt11: Some(String::from("lnbc21u1p0test")),
            }
        );
        assert_eq!(
            parsed.to_tag(),
            Tag::parse(["amount", "21000", "lnbc21u1p0test"]).unwrap()
        );
    }

    #[test]
    fn test_nip57_anon_tag() {
        let tag = vec!["anon", "encrypted-message"];
        let parsed = Nip57Tag::parse(tag).unwrap();

        assert_eq!(
            parsed,
            Nip57Tag::Anon {
                msg: Some(String::from("encrypted-message")),
            }
        );
        assert_eq!(
            parsed.to_tag(),
            Tag::parse(["anon", "encrypted-message"]).unwrap()
        );
    }

    #[test]
    fn test_encrypt_decrypt_private_zap_message() {
        let alice_keys = Keys::generate();
        let bob_keys = Keys::generate();

        let relays = [RelayUrl::parse("wss://relay.damus.io").unwrap()];
        let msg = "Private Zap message!";
        let data = ZapRequestData::new(bob_keys.public_key(), relays).message(msg);
        let private_zap = private_zap_request(data, &alice_keys).unwrap();

        let private_zap_msg = decrypt_sent_private_zap_message(
            alice_keys.secret_key(),
            &bob_keys.public_key(),
            &private_zap,
        )
        .unwrap();

        assert_eq!(msg, &private_zap_msg.content);

        let private_zap_msg =
            decrypt_received_private_zap_message(bob_keys.secret_key(), &private_zap).unwrap();

        assert_eq!(msg, &private_zap_msg.content)
    }
}
