#![allow(clippy::needless_lifetimes)]

mod address;
mod agent;
mod connection;
mod dns_pattern;
mod error;
mod tls;

pub mod config;
pub mod setup;

/// Version of this crate.
fn version() -> Result<protocol::Version, Error> {
    let parse = |s: &str| s.parse().map_err(|e| Error::Other(Box::new(e)));
    let major = parse(env!("CARGO_PKG_VERSION_MAJOR"))?;
    let minor = parse(env!("CARGO_PKG_VERSION_MINOR"))?;
    let patch = parse(env!("CARGO_PKG_VERSION_PATCH"))?;
    Ok(protocol::Version::new(major, minor, patch))
}

use minicbor_io::{AsyncReader, AsyncWriter};
use tokio::io;
use tokio::net::TcpStream;
use tokio_util::compat::Compat;

pub(crate) type Reader = AsyncReader<Compat<io::ReadHalf<tls::Stream<TcpStream>>>>;
pub(crate) type Writer = AsyncWriter<Compat<io::WriteHalf<tls::Stream<TcpStream>>>>;

pub use self::agent::Agent;
pub use self::config::{Config, Options};
pub use self::dns_pattern::DnsPattern;
pub use error::Error;

