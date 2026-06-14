#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod core;
pub mod display;
pub mod errors;
pub mod hooks;
#[cfg(feature = "java")]
pub mod minecraft;
pub mod protocols;

pub mod macros;

pub use crate::core::router::EnderRouter;
pub use crate::core::types::{EnderConfig, EnderRoute, LifecycleHandler, ServerProxy, StartupOn};
pub use crate::errors::{EnderError, Result};
