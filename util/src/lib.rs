pub mod base64;
pub mod crypto;
pub mod io;
pub mod serde;
pub mod time;

use ::serde::de::{self, Deserialize, Deserializer};
use ::serde::Serialize;
use std::fmt;
use std::convert::TryFrom;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct NonEmpty<T>(Vec<T>);

impl<T> NonEmpty<T> {
    pub fn new(val: T) -> Self {
        NonEmpty(vec![val])
    }
}

impl<T> From<NonEmpty<T>> for Vec<T> {
    fn from(ne: NonEmpty<T>) -> Self {
        ne.0
    }
}

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = Empty;

    fn try_from(v: Vec<T>) -> Result<Self, Self::Error> {
        if v.is_empty() {
            return Err(Empty(()))
        }
        Ok(NonEmpty(v))
    }
}

impl<T> Deref for NonEmpty<T> {
    type Target = Vec<T>;

   fn deref(&self) -> &Self::Target {
       &self.0
   }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NonEmpty<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let val = <Vec<T>>::deserialize(d)?;
        NonEmpty::try_from(val).map_err(|_| de::Error::custom("empty vector"))
    }
}

/// Log the error and exit the process with code 1.
pub fn exit<T, D>(context: &'static str) -> impl FnOnce(D) -> T
where
    D: std::fmt::Display
{
    move |e| {
        eprintln!("{}: {}", context, e);
        std::process::exit(1)
    }
}

/// Error type used by `TryFrom` impl of [`NonEmpty`].
#[derive(Clone, Debug)]
pub struct Empty(());

impl fmt::Display for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("empty")
    }
}

impl std::error::Error for Empty {}

