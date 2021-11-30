use crate::Error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io;
use tokio::net::TcpStream;
use tokio_rustls::rustls::{self, ClientConfig};
use tokio_rustls::TlsConnector;
use util::HostName;

pub use tokio_rustls::client::TlsStream as Stream;

/// A TLS client.
#[derive(Clone)]
pub struct Client {
    config: Arc<ClientConfig>
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("tls client config")
    }
}

impl Client {
    /// Create a new TLS client.
    pub fn new(config: &crate::Config) -> Result<Self, Error> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS
            .0
            .iter()
            .map(|ta| {
                rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            })
        );

        if let Some(certs) = &config.server.trust {
            for c in certs.iter() {
                root_store.add(c)?
            }
        }

        let cfg = ClientConfig::builder()
            .with_cipher_suites(&[rustls::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256])
            .with_kx_groups(&[&rustls::kx_group::X25519])
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Client { config: Arc::new(cfg) })
    }

    /// Connect with this client to the given address.
    ///
    /// Server name is checked against the given hostname.
    pub async fn connect(&self, addr: SocketAddr, hostname: &HostName) -> io::Result<Stream<TcpStream>> {
        let conn = TlsConnector::from(self.config.clone());
        let sock = TcpStream::connect(&addr).await?;
        conn.connect(hostname.as_server_name().clone(), sock).await
    }

    /// Connect to any of the given addresses.
    ///
    /// Server name is checked against the given hostname.
    pub async fn connect_any<I>(&self, iter: I, hostname: &HostName) -> io::Result<Stream<TcpStream>>
    where
        I: Iterator<Item = SocketAddr>
    {
        let host: &str = hostname.as_str();

        for addr in iter {
            match self.connect(addr, hostname).await {
                Ok(s)  => return Ok(s),
                Err(e) => log::debug!("failed to connect to {} ({}): {}", addr, host, e)
            }
        }

        let msg = format!("could not connect to any address of {}", host);
        Err(io::Error::new(io::ErrorKind::AddrNotAvailable, msg))
    }
}
