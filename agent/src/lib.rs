#![allow(clippy::needless_lifetimes)]

mod address;
mod agent;
mod dns_pattern;
mod error;
mod stream;
mod tls;

pub mod config;

/// Version of this crate.
pub fn version() -> Result<protocol::Version, Error> {
    let parse = |s: &str| s.parse().map_err(|e| Error::Version(Box::new(e)));
    let major = parse(env!("CARGO_PKG_VERSION_MAJOR"))?;
    let minor = parse(env!("CARGO_PKG_VERSION_MINOR"))?;
    let patch = parse(env!("CARGO_PKG_VERSION_PATCH"))?;
    Ok(protocol::Version::new(major, minor, patch))
}

use futures::io;
use minicbor_io::{AsyncReader, AsyncWriter};

pub(crate) type Reader = AsyncReader<io::ReadHalf<yamux::Stream>>;
pub(crate) type Writer = AsyncWriter<io::WriteHalf<yamux::Stream>>;

pub use self::agent::Agent;
pub use self::config::{Config, Options};
pub use self::dns_pattern::DnsPattern;
pub use error::Error;

