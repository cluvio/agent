use crate::dns_pattern::DnsPattern;
use sealed_boxes::SecretKey;
use serde::{Deserialize, Deserializer};
use serde::de::{self, IntoDeserializer};
use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;
use std::{fmt, fs, io};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
use tokio_rustls::rustls::Certificate;
use util::NonEmpty;

pub use ipnet::{IpNet, Ipv4Net, Ipv6Net};
pub use tokio_rustls::webpki::{DNSName, DNSNameRef};

/// Command-line options.
#[derive(Debug, StructOpt)]
#[non_exhaustive]
#[structopt(name = "cluvio-agent")]
pub struct Options {
    /// Path to configuration file.
    #[structopt(short, long, parse(from_os_str), required_unless_one(&["gen-keypair", "setup", "version"]))]
    pub config: Option<PathBuf>,

    /// Show version information.
    #[structopt(long)]
    pub version: bool,

    /// Log-level.
    #[structopt(short, long)]
    pub log: Option<String>,

    /// Use json format for log messages.
    #[structopt(short, long)]
    pub json: bool
}

/// Config file representation.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Config {
    /// The base64-encoded private key of this agent.
    #[serde(deserialize_with = "util::serde::decode_secret_key")]
    pub private_key: SecretKey,

    /// The max. number of data connections to open (default = 100).
    #[serde(default = "default_max_connections")]
    pub max_connections: u16,

    /// The timeout of connects.
    #[serde(deserialize_with = "util::serde::decode_duration", default = "default_connect_timeout")]
    pub connect_timeout: Duration,

    /// How often to check if the control server is still there.
    #[serde(deserialize_with = "util::serde::decode_duration", default = "default_ping_frequency")]
    pub ping_frequency: Duration,

    /// List of allowed external domains or IPv4/IPv6 networks (per default there are no constraints).
    #[serde(default = "default_net", rename = "allowed-external")]
    pub external: NonEmpty<Network>,

    /// List of allowed internal domains or IPv4/IPv6 networks (per default there are no constraints).
    #[serde(default = "default_net", rename = "allowed-internal")]
    pub internal: NonEmpty<Network>,

    /// Control server settings.
    #[serde(rename = "control-server")]
    pub control: ControlServer
}

#[derive(Debug, Clone)]
pub enum Network {
    /// IP network.
    Ip(IpNet),
    /// A DNS name.
    Dns(DNSName),
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
        if let Ok(dns) = DNSNameRef::try_from_ascii_str(&s).map(DNSName::from) {
            return Ok(Network::Dns(dns))
        }
        if let Ok(pat) = DnsPattern::try_from(s.borrow()) {
            return Ok(Network::Pat(pat))
        }
        Err(de::Error::custom("network syntax error; neither IP address nor DNS name (pattern)"))
    }
}

impl Config {
    pub fn new(sk: SecretKey, host: DNSName, port: u16) -> Self {
        Config {
            private_key: sk,
            max_connections: default_max_connections(),
            connect_timeout: default_connect_timeout(),
            ping_frequency: default_ping_frequency(),
            external: default_net(),
            internal: default_net(),
            control: ControlServer { host, port, trust: None }
        }
    }

    pub fn read_file(p: impl AsRef<Path>) -> io::Result<Self> {
        let f = fs::read(p)?;
        let c = toml::from_slice(&f)?;
        Ok(c)
    }

    pub fn control_mut(&mut self) -> &mut ControlServer {
        &mut self.control
    }

    pub fn external_mut(&mut self) -> &mut NonEmpty<Network> {
        &mut self.external
    }

    pub fn internal_mut(&mut self) -> &mut NonEmpty<Network> {
        &mut self.internal
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("private_key", &"********")
            .field("max_connections", &self.max_connections)
            .field("connect_timeout", &self.connect_timeout)
            .field("ping_frequency", &self.ping_frequency)
            .field("control_server", &self.control)
            .field("external", &self.external)
            .field("internal", &self.internal)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ControlServer {
    /// The hostname of the remote control server.
    #[serde(deserialize_with = "util::serde::decode_dns_name")]
    pub host: DNSName,

    /// The port to connect to (default = 443).
    #[serde(default = "default_port")]
    pub port: u16,

    /// Optional certificate to add as trusted.
    #[serde(deserialize_with = "util::serde::decode_opt_certificates", default)]
    pub trust: Option<NonEmpty<Certificate>>
}

fn default_max_connections() -> u16 {
    1000
}

fn default_port() -> u16 {
    443
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_ping_frequency() -> Duration {
    Duration::from_secs(30)
}

fn default_net() -> NonEmpty<Network> {
    let v = vec![
        Network::Ip(Ipv4Net::new([0,0,0,0].into(), 0).expect("valid network").into()),
        Network::Ip(Ipv6Net::new([0,0,0,0,0,0,0,0].into(), 0).expect("valid network").into()),
        Network::Pat(DnsPattern::wildcard())
    ];
    NonEmpty::try_from(v).expect("3 element vector is not empty")
}

