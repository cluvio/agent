use minicbor::{Encode, Decode};
use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

/// A UNIX timestamp, i.e. seconds since 1970-01-01 00:00:00 UTC.
#[derive(Copy, Clone, Debug, Decode, Encode, PartialEq, Eq, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct UnixTime(#[n(0)] u64);

impl UnixTime {
    pub fn now() -> Result<Self, SystemTimeError> {
        let d = SystemTime::now().duration_since(UNIX_EPOCH)?;
        Ok(UnixTime::from(d))
    }

    pub fn seconds(self) -> u64 {
        self.0
    }
}

impl From<u64> for UnixTime {
    fn from(s: u64) -> Self {
        UnixTime(s)
    }
}

impl From<Duration> for UnixTime {
    fn from(d: Duration) -> Self {
        UnixTime(d.as_secs())
    }
}

