use protocol::{Id, Reason};
use std::io;
use thiserror::Error;
use tokio::time::error::Elapsed;
use tokio_rustls::{rustls, webpki};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("cbor error: {0}")]
    Cbor(#[from] minicbor_io::Error),

    #[error("crypto error: {0}")]
    Crypto(#[from] sealed_boxes::Error),

    #[error("certificate error: {0}")]
    Pki(#[from] webpki::Error),

    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("timeout: {0}")]
    Timeout(#[from] Elapsed),

    #[error("host {0} not reachable")]
    Unreachable(String),

    #[error("agent is terminated, reason: {0:?}")]
    Terminated(Reason),

    #[error("multiplex error: {0}")]
    Yamux(#[from] yamux::ConnectionError),

    #[error("invalid version: {0}")]
    Version(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("unknown message type: {0}")]
    UnknownMessageType(Id)
}

