use crate::Error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io;
use tokio::net::TcpStream;
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

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
        let mut cfg = ClientConfig::new();
        cfg.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        if let Some(certs) = &config.server.trust {
            for c in certs.iter() {
                cfg.root_store.add(c)?
            }
        }
        Ok(Client { config: Arc::new(cfg) })
    }

    /// Connect with this client to the given address.
    ///
    /// Server name is checked against the given hostname.
    pub async fn connect(&self, addr: SocketAddr, hostname: DNSNameRef<'_>) -> io::Result<Stream<TcpStream>> {
        let conn = TlsConnector::from(self.config.clone());
        let sock = TcpStream::connect(&addr).await?;
        conn.connect(hostname, sock).await
    }

    /// Connect to any of the given addresses.
    ///
    /// Server name is checked against the given hostname.
    pub async fn connect_any<I>(&self, iter: I, hostname: DNSNameRef<'_>) -> io::Result<Stream<TcpStream>>
    where
        I: Iterator<Item = SocketAddr>
    {
        let host: &str = hostname.into();

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
