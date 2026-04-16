#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

pub mod core;
pub mod errors;
pub mod hooks;
pub mod protocols;

pub use crate::core::router::EnderRouter;
pub use crate::errors::{EnderError, Result};
