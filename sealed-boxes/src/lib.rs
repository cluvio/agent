//! Encryption and decryption of small array based messages.
//!
//! The algorithm used corresponds to [`crypto_box_sealed`][1]:
//!
//! `ephemeral_pk || box(m, recipient_pk, ephemeral_sk, nonce=blake2b(ephemeral_pk || recipient_pk))`
//!
//! [1]: https://doc.libsodium.org/public-key_cryptography/sealed_boxes

use crypto_box::{ChaChaBox, aead::AeadInPlace};
use minicbor::{Decode, Encode};
use rand_core::{OsRng, RngCore};
use std::convert::TryInto;

pub use crypto_box::{PublicKey, SecretKey, aead::Error};

/// {public, secret} key lengths
const K: usize = 32;
/// tag length
const T: usize = 16;

/// A triple of public key, payload data and tag.
///
/// This is the actual data exchanged between peers. The key is the ephemeral
/// public key whose corresponding private key was used to encrypt the data.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Data<const N: usize> {
    #[n(0)]
    #[cbor(with = "minicbor::bytes")]
    pub key: [u8; K],

    #[n(1)]
    #[cbor(with = "minicbor::bytes")]
    pub data: [u8; N],

    #[n(2)]
    #[cbor(with = "minicbor::bytes")]
    pub tag: [u8; T]
}

/// Generate a new random secret key.
pub fn gen_secret_key() -> SecretKey {
    SecretKey::from(fresh_array())
}

/// Generate a new random array.
pub fn fresh_array<const N: usize>() -> [u8; N] {
    let mut a = [0; N];
    OsRng.fill_bytes(&mut a);
    a
}

/// Encrypt a message for the given public key.
pub fn encrypt<const N: usize>(pk: &PublicKey, mut msg: [u8; N]) -> Result<Data<N>, Error> {
    let es = gen_secret_key();
    let ep = es.public_key();
    let nc = nonce(ep.as_bytes(), pk.as_bytes()).into();
    let cb = ChaChaBox::new(pk, &es);
    let tg = cb.encrypt_in_place_detached(&nc, &[], &mut msg[..])?;
    Ok(Data { key: *ep.as_bytes(), data: msg, tag: tg.into() })
}

/// Decrypt a message using the given secret key.
pub fn decrypt<const N: usize>(sk: &SecretKey, mut data: Data<N>) -> Result<[u8; N], Error> {
    let ep = PublicKey::from(data.key);
    let tg = data.tag.into();
    let nc = nonce(ep.as_bytes(), sk.public_key().as_bytes()).into();
    let cb = ChaChaBox::new(&ep, sk);
    cb.decrypt_in_place_detached(&nc, &[], &mut data.data, &tg)?;
    Ok(data.data)
}

/// Calculate the nonce as `blake2b(a || b)`.
fn nonce<const N: usize>(a: &[u8], b: &[u8]) -> [u8; N] {
    let mut s = blake2b_simd::Params::new().hash_length(N).to_state();
    s.update(a);
    s.update(b);
    let h = s.finalize();
    h.as_bytes().try_into().expect("hash length = N")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        let da = fresh_array::<57>();
        let sk = gen_secret_key();
        let pk = sk.public_key();
        let it = encrypt(&pk, da).unwrap();
        {
            let v = minicbor::to_vec(&it).unwrap();
            let d: Data<57> = minicbor::decode(&v).unwrap();
            assert_eq!(d, it)
        }
        let db = decrypt(&sk, it).unwrap();
        assert_eq!(da, db)
    }

    #[test]
    fn failure() {
        let sk1 = gen_secret_key();
        let sk2 = gen_secret_key();
        let pk1 = sk1.public_key();
        let dat = encrypt(&pk1, fresh_array::<57>()).unwrap();
        {
            let v = minicbor::to_vec(&dat).unwrap();
            let d: Data<57> = minicbor::decode(&v).unwrap();
            assert_eq!(d, dat)
        }
        assert!(decrypt(&sk2, dat).is_err())
    }
}
