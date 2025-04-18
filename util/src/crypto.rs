use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::{AeadInPlace, Error, KeyInit};
use minicbor::{Decode, Encode};
use minicbor::decode::{self, Decoder};
use minicbor::encode::{self, Encoder, Write};
use rand_core::TryRngCore;
use std::convert::TryFrom;

#[derive(Clone)]
pub struct Key(chacha20poly1305::Key);

#[derive(Debug, Clone, Copy)]
pub struct Nonce(chacha20poly1305::XNonce);

impl Nonce {
    pub fn fresh() -> Self {
        let mut n = [0; 24];
        rand_core::OsRng.try_fill_bytes(&mut n).expect("OS RNG not available or misconfigured");
        Nonce::from(n)
    }
}

impl Key {
    pub fn fresh() -> Self {
        let mut k = [0; 32];
        rand_core::OsRng.try_fill_bytes(&mut k).expect("OS RNG not available or misconfigured");
        Key::from(k)
    }

    pub fn encrypt(&self, n: &Nonce, ad: &[u8], val: &mut Vec<u8>) -> Result<(), Error> {
        let x = XChaCha20Poly1305::new(&self.0);
        x.encrypt_in_place(&n.0, ad, val)
    }

    pub fn decrypt(&self, n: &Nonce, ad: &[u8], val: &mut Vec<u8>) -> Result<(), Error> {
        let x = XChaCha20Poly1305::new(&self.0);
        x.decrypt_in_place(&n.0, ad, val)
    }
}

impl From<[u8; 32]> for Key {
    fn from(k: [u8; 32]) -> Self {
        Key(k.into())
    }
}

impl From<[u8; 24]> for Nonce {
    fn from(k: [u8; 24]) -> Self {
        Nonce(k.into())
    }
}

impl<C> Encode<C> for Nonce {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, _: &mut C) -> Result<(), encode::Error<W::Error>> {
        e.bytes(&self.0)?.ok()
    }
}

impl<'b, C> Decode<'b, C> for Nonce {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, decode::Error> {
        let p = d.position();
        let b = d.bytes()?;
        let a = <[u8; 24]>::try_from(b).map_err(|_| {
            decode::Error::message("crypto::Nonce not 24 bytes").at(p)
        })?;
        Ok(Nonce::from(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        let k = Key::fresh();
        let n = Nonce::fresh();
        let a = &[1,2,3,4,5];
        let mut v = b"hello world".to_vec();
        k.encrypt(&n, a, &mut v).unwrap();
        assert_ne!(&b"hello world"[..], &v);
        k.decrypt(&n, a, &mut v).unwrap();
        assert_eq!(&b"hello world"[..], &v)
    }

}
