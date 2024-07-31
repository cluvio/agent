use crate::dns_pattern::DnsPattern;
use sealed_boxes::SecretKey;
use serde::{Deserialize, Deserializer};
use serde::de::{self, IntoDeserializer};
use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio_rustls::rustls::pki_types::CertificateDer;
use util::{HostName, NonEmpty};

pub use ipnet::{IpNet, Ipv4Net, Ipv6Net};

/// Command-line options.
#[derive(Debug, clap::Parser)]
#[non_exhaustive]
#[command(name = "cluvio-agent")]
pub struct Options {
    /// Path to the configuration file.
    ///
    /// If this option is not present, a config file named `cluvio-agent.toml` is looked
    /// for in various locations.
    ///
    /// Unix:
    ///   1. In the directory of the `cluvio-agent` executable.
    ///   2. In `$HOME/.config`.
    ///   3. In `/etc`.
    ///
    /// Mac:
    ///   1. In the directory of the `cluvio-agent` executable.
    ///   2. In `$HOME`.
    ///   3. In `/etc`.
    ///
    /// Windows:
    ///   1. In `%USERPROFILE%\AppData\Roaming` (`%APPDATA%`).
    ///   2. In the directory of the `cluvio-agent` executable.
    #[arg(short, long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// Show version information.
    #[arg(long)]
    pub version: bool,

    /// Log-level.
    #[arg(short, long)]
    pub log: Option<String>,

    /// Use json format for log messages.
    #[arg(short, long)]
    pub json: bool,

    /// Generate a new keypair.
    #[arg(short, long)]
    pub gen_keypair: bool
}

/// Config file representation.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Config {
    /// The base64-encoded private key of this agent.
    #[serde(deserialize_with = "util::serde::decode_secret_key")]
    pub secret_key: SecretKey,

    /// The timeout of connects.
    #[serde(deserialize_with = "util::serde::decode_duration", default = "default_connect_timeout")]
    pub connect_timeout: Duration,

    /// How often to check if the server is still there.
    #[serde(deserialize_with = "util::serde::decode_duration", default = "default_ping_frequency")]
    pub ping_frequency: Duration,

    /// List of allowed domains or IPv4/IPv6 networks (per default there are no constraints).
    #[serde(default = "default_net")]
    pub allowed_addresses: NonEmpty<Network>,

    /// Server settings.
    pub server: Server
}

#[derive(Debug, Clone)]
pub enum Network {
    /// IP network.
    Ip(IpNet),
    /// A DNS name.
    Dns(HostName),
    /// A DNS name pattern.
    Pat(DnsPattern),
}

impl TryFrom<&str> for Network {
    type Error = serde::de::value::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Network::deserialize(s.into_deserializer())
    }
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <Cow<'de, str>>::deserialize(d)?;
        if let Ok(net) = IpNet::from_str(&s) {
            return Ok(Network::Ip(net))
        }
        if let Ok(dns) = HostName::try_from(&*s) {
            return Ok(Network::Dns(dns))
        }
        if let Ok(pat) = DnsPattern::try_from(s.borrow()) {
            return Ok(Network::Pat(pat))
        }
        Err(de::Error::custom("network syntax error; neither IP address nor DNS name (pattern)"))
    }
}

impl Config {
    pub fn new(sk: SecretKey, host: HostName, port: u16) -> Self {
        Config {
            secret_key: sk,
            connect_timeout: default_connect_timeout(),
            ping_frequency: default_ping_frequency(),
            allowed_addresses: default_net(),
            server: Server { host, port, trust: None }
        }
    }

    pub fn server_mut(&mut self) -> &mut Server {
        &mut self.server
    }

    pub fn allowed_addresses_mut(&mut self) -> &mut NonEmpty<Network> {
        &mut self.allowed_addresses
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("secret_key", &"********")
            .field("connect_timeout", &self.connect_timeout)
            .field("ping_frequency", &self.ping_frequency)
            .field("server", &self.server)
            .field("allowed_addresses", &self.allowed_addresses)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct Server {
    /// The hostname of the remote server.
    pub host: HostName,

    /// The port to connect to (default = 443).
    #[serde(default = "default_port")]
    pub port: u16,

    /// Optional certificate to add as trusted.
    #[serde(deserialize_with = "util::serde::decode_opt_certificates", default)]
    pub trust: Option<NonEmpty<CertificateDer<'static>>>
}

fn default_port() -> u16 {
    443
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_ping_frequency() -> Duration {
    Duration::from_secs(60)
}

fn default_net() -> NonEmpty<Network> {
    let v = vec![
        Network::Ip(Ipv4Net::new([0,0,0,0].into(), 0).expect("valid network").into()),
        Network::Ip(Ipv6Net::new([0,0,0,0,0,0,0,0].into(), 0).expect("valid network").into()),
        Network::Pat(DnsPattern::wildcard())
    ];
    NonEmpty::try_from(v).expect("3 element vector is not empty")
}
