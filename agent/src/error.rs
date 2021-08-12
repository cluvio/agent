use protocol::{Id, Reason};
use std::io;
use thiserror::Error;
use tokio::time::error::Elapsed;
use tokio_rustls::webpki::{self, InvalidDNSNameError};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("cbor error: {0}")]
    Cbor(#[from] minicbor_io::Error),

    #[error("crypto error: {0}")]
    Crypto(#[from] sealed_boxes::Error),

    #[error("invalid dns name: {0}")]
    Dns(#[from] InvalidDNSNameError),

    #[error("certificate error: {0}")]
    Pki(#[from] webpki::Error),

    #[error("timeout: {0}")]
    Timeout(#[from] Elapsed),

    #[error("unexpected {0}, was expecting {1}")]
    Unexpected(&'static str, &'static str),

    #[error("host {0} not reachable")]
    Unreachable(String),

    #[error("id mismatch {expected} != {actual}")]
    Mismatch {
        expected: Id,
        actual: Id
    },

    #[error("agent is terminated, reason: {0:?}")]
    Terminated(Reason),

    #[error("other error: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send>)
}

