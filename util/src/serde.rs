use crate::NonEmpty;
use crate::crypto;
use sealed_boxes::SecretKey;
use serde::{Deserialize, Deserializer, de::Error};
use serde::{Serialize, Serializer};
use std::borrow::{Borrow, Cow};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use tokio_rustls::rustls::{internal::pemfile, Certificate, PrivateKey};
use tokio_rustls::webpki::{DNSName, DNSNameRef};

/// Deserialize any `FromStr` impl.
pub fn decode_from_str<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Debug
{
    let s = <Cow<'de, str>>::deserialize(d)?;
    s.parse().map_err(|e| Error::custom(format!("{:?}", e)))
}

/// Deserialize DNS name.
pub fn decode_dns_name<'de, D: Deserializer<'de>>(d: D) -> Result<DNSName, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    DNSNameRef::try_from_ascii_str(s.borrow())
        .map(DNSName::from)
        .map_err(|e| {
            Error::custom(format!("invalid dns name: {}", e))
        })
}

/// Serialize DNS name.
pub fn encode_dns_name<S: Serializer>(dns: &DNSName, ser: S) -> Result<S::Ok, S::Error> {
    <&str>::from(dns.as_ref()).serialize(ser)
}

/// Deserialize human-friendly duration value.
pub fn decode_duration<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    humantime::parse_duration(s.borrow()).map_err(|e| {
        Error::custom(format!("invalid duration: {}", e))
    })
}

/// Serialize human-friendly duration value.
pub fn encode_duration<S: Serializer>(d: &Duration, ser: S) -> Result<S::Ok, S::Error> {
    humantime::format_duration(*d).to_string().serialize(ser)
}

/// Deserialize base64-encoded private key.
#[allow(clippy::redundant_closure)]
pub fn decode_secret_key<'de, D: Deserializer<'de>>(d: D) -> Result<SecretKey, D::Error> {
    decode_base64(d)?
        .try_into()
        .map(|a: [u8; 32]| SecretKey::from(a))
        .map_err(|_| Error::custom("invalid length"))
}

/// Serialize private key as base64-encoded string.
pub fn encode_secret_key<S: Serializer>(sk: &SecretKey, ser: S) -> Result<S::Ok, S::Error> {
    let b64 = crate::base64::encode(sk.to_bytes());
    ser.serialize_str(&b64)
}

/// Deserialize base64-encoded string.
pub fn decode_base64<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    crate::base64::decode(s.borrow()).ok_or_else(|| Error::custom("invalid base64"))
}

/// Decode base64-encoded string as bytes array.
pub fn decode_base64_array<'de, D, const N: usize>(d: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>
{
    let v = decode_base64(d)?;
    <[u8; N]>::try_from(v).map_err(|_| Error::custom("invalid array length"))
}

/// Decode a base64-encoded, symmetric encryption key.
pub fn decode_crypto_key<'de, D: Deserializer<'de>>(d: D) -> Result<crypto::Key, D::Error> {
    decode_base64_array(d).map(crypto::Key::from)
}

pub fn decode_private_key<'de, D: Deserializer<'de>>(d: D) -> Result<PrivateKey, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    let v = pemfile::pkcs8_private_keys(&mut s.as_bytes())
        .map_err(|()| {
            Error::custom("failed to read private key")
        })?;
    if v.len() > 1 {
        return Err(Error::custom("multiple private keys are not supported"))
    }
    if let Some(k) = v.into_iter().next() {
        Ok(k)
    } else {
        Err(Error::custom("no private key found"))
    }
}

pub fn decode_certificates<'de, D: Deserializer<'de>>(d: D) -> Result<NonEmpty<Certificate>, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    let v = pemfile::certs(&mut s.as_bytes())
        .map_err(|()| {
            Error::custom("failed to read certificate")
        })?;
    NonEmpty::try_from(v).map_err(|_| Error::custom("no certificate found"))
}

pub fn decode_opt_certificates<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NonEmpty<Certificate>>, D::Error> {
    if let Some(s) = <Option<Cow<'de, str>>>::deserialize(d)? {
        let v = pemfile::certs(&mut s.as_bytes())
            .map_err(|()| {
                Error::custom("failed to read certificate")
            })?;
        NonEmpty::try_from(v)
            .map(Some)
            .map_err(|_| Error::custom("no certificate found"))
    } else {
        Ok(None)
    }
}

pub fn decode_nonempty<'de, T, D>(d: D) -> Result<NonEmpty<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>
{
    let v = <Vec<T>>::deserialize(d)?;
    if v.is_empty() {
        return Err(Error::custom("attempt to construct an empty `NonEmpty`"))
    }
    Ok(NonEmpty(v))
}

