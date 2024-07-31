use crate::NonEmpty;
use crate::crypto;
use sealed_boxes::SecretKey;
use serde::{Deserialize, Deserializer, de::Error};
use serde::{Serialize, Serializer};
use std::borrow::{Borrow, Cow};
use std::convert::{TryFrom, TryInto};
use std::{io, fmt};
use std::str::FromStr;
use std::time::Duration;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

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
    let b64 = crate::base64::encode(sk.as_bytes());
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

/// Decode PEM-encoded private key.
pub fn decode_private_key<'de, D: Deserializer<'de>>(d: D) -> Result<PrivatePkcs8KeyDer<'static>, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    let v = rustls_pemfile::pkcs8_private_keys(&mut s.as_bytes())
        .collect::<Result<Vec<PrivatePkcs8KeyDer<'static>>, io::Error>>()
        .map_err(|e| {
            Error::custom(format!("failed to read private key: {}", e))
        })?;
    if v.len() > 1 {
        return Err(Error::custom("multiple private keys are not supported"))
    }
    if let Some(k) = v.into_iter().next() {
        Ok(PrivatePkcs8KeyDer::from(k))
    } else {
        Err(Error::custom("no private key found"))
    }
}

/// Decode PEM-encoded certificates.
pub fn decode_certificates<'de, D: Deserializer<'de>>(d: D) -> Result<NonEmpty<CertificateDer<'static>>, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    let v = rustls_pemfile::certs(&mut s.as_bytes())
        .collect::<Result<Vec<CertificateDer<'static>>, io::Error>>()
        .map_err(|e| {
            Error::custom(format!("failed to read certificate: {}", e))
        })?;
    NonEmpty::try_from(v).map_err(|_| Error::custom("no certificate found"))
}

/// Decode optional PEM-encoded certificates.
pub fn decode_opt_certificates<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NonEmpty<CertificateDer<'static>>>, D::Error> {
    if let Some(s) = <Option<Cow<'de, str>>>::deserialize(d)? {
        let v = rustls_pemfile::certs(&mut s.as_bytes())
            .collect::<Result<Vec<CertificateDer<'static>>, io::Error>>()
            .map_err(|e| {
                Error::custom(format!("failed to read certificate: {}", e))
            })?;
        NonEmpty::try_from(v)
            .map(Some)
            .map_err(|_| Error::custom("no certificate found"))
    } else {
        Ok(None)
    }
}

