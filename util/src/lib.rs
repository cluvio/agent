pub mod base64;
pub mod crypto;
pub mod io;
pub mod serde;
pub mod time;

use ::serde::de::{self, Deserialize, Deserializer};
use ::serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;
use std::convert::TryFrom;
use std::ops::Deref;
use std::str::FromStr;
use tokio_rustls::rustls::pki_types::ServerName;

/// A non-empty vector.
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

#[derive(Clone, Copy, Debug)]
pub enum Location {
    Eu,
    Us
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Eu => f.write_str("eu"),
            Location::Us => f.write_str("us")
        }
    }
}

/// Error caused by parsing invalid or unknown locations.
#[derive(Clone, Debug)]
pub struct InvalidLocation(String);

impl fmt::Display for InvalidLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid location: {}", self.0)
    }
}

impl std::error::Error for InvalidLocation {}

impl<'a> FromStr for Location {
    type Err = InvalidLocation;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eu" => Ok(Location::Eu),
            "us" => Ok(Location::Us),
            _    => Err(InvalidLocation(s.into()))
        }
    }
}

#[derive(Debug, Clone)]
pub struct HostName(ServerName<'static>);

impl HostName {
    pub fn as_str(&self) -> &str {
        if let ServerName::DnsName(n) = &self.0 {
            return n.as_ref()
        }
        unreachable!()
    }

    pub fn as_server_name(&self) -> &ServerName<'static> {
        &self.0
    }
}

impl PartialEq for HostName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for HostName {}

impl fmt::Display for HostName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let ServerName::DnsName(n) = &self.0 {
            return f.write_str(n.as_ref())
        }
        unreachable!()
    }
}

#[derive(Clone, Debug)]
pub struct InvalidHostName(String);

impl fmt::Display for InvalidHostName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid hostname: {}", self.0)
    }
}

impl std::error::Error for InvalidHostName {}

impl<'a> FromStr for HostName {
    type Err = InvalidHostName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match ServerName::try_from(s.to_string()) {
            Ok(n@ServerName::DnsName(_)) => Ok(HostName(n)),
            Ok(_)  => Err(InvalidHostName(format!("not a DNS name: `{}`", s))),
            Err(e) => Err(InvalidHostName(format!("`{}`: {:?}", s, e)))
        }
    }
}

impl TryFrom<&str> for HostName {
    type Error = InvalidHostName;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        HostName::from_str(s)
    }
}

impl<'de> Deserialize<'de> for HostName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <Cow<'de, str>>::deserialize(d)?;
        HostName::try_from(&*s).map_err(de::Error::custom)
    }
}

impl Serialize for HostName {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(s)
    }
}
