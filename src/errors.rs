use std::{io, net::AddrParseError, result::Result as StdResult};

use refractium::RefractiumError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum EnderError {
    #[error("Refractium internal error: {0}")]
    Refractium(#[from] RefractiumError),

    #[error("IO error occurred: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse socket address: {0}")]
    AddrParse(#[from] AddrParseError),

    #[error("No backend available for protocol: {0}")]
    NoBackend(String),

    #[error("Configuration error ({0}): {1}")]
    Config(String, String),

    #[error("Auto-wakeup failed for {0}: {1}")]
    WakeupFailure(String, String),

    #[error("Unknown error occurred")]
    Unknown,
}

pub type Result<T> = StdResult<T, EnderError>;
