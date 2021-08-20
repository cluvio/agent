use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("status error: {0}")]
    Status(reqwest::StatusCode),

    #[error("no version found")]
    NoVersion,

    #[error("invalid version: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("{0} not an xz archive")]
    NoXzExt(String),

    #[error("update failed")]
    UpdateFailed
}
