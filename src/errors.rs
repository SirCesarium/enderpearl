use std::{io, net::AddrParseError, result::Result as StdResult};

use refractium::RefractiumError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum EnderError {
    /// Wraps an error from the underlying `Refractium` engine.
    #[error("Refractium internal error: {0}")]
    Refractium(#[from] RefractiumError),

    /// Wraps a standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Wraps a socket address parse error.
    #[error("Failed to parse socket address: {0}")]
    AddrParse(#[from] AddrParseError),

    /// No backend server is available for a protocol route.
    #[error("No backend available for protocol: {0}")]
    NoBackend(String),

    /// A configuration value is invalid or references a disabled feature.
    #[error("Configuration error ({0}): {1}")]
    Config(String, String),

    /// A Minecraft packet could not be parsed.
    #[error("Packet parse error: {0}")]
    PacketParse(String),

    /// Connecting to or proxying to a backend target failed.
    #[error("Proxy error: {0}")]
    Proxy(String),

    /// The auto-wakeup command failed to execute.
    #[error("Auto-wakeup failed for {0}: {1}")]
    WakeupFailure(String, String),
}

pub type Result<T> = StdResult<T, EnderError>;
