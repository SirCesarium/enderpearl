use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::errors::Result;
use refractium::RefractiumProtocol;

pub type AsyncResultFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub trait LifecycleHandler: Send + Sync {
    fn on_startup(&self) -> AsyncResultFuture;
    fn on_shutdown(&self) -> AsyncResultFuture;
}

/// Implement this to handle connections for a protocol when the backend is offline.
///
/// Return a TCP listener bound to `127.0.0.1:0` — the port it actually
/// binds to is returned and used in the route table so refractium forwards
/// matching traffic to your proxy instead of directly to the backend.
pub trait ServerProxy: Send + Sync {
    fn serve(self: Arc<Self>) -> Pin<Box<dyn Future<Output = Result<u16>> + Send>>;
}

pub struct EnderConfig {
    pub bind: String,
    pub port: u16,
    pub peek_buffer_size: usize,
    pub peek_timeout_ms: u64,
    pub upstreams: Vec<EnderRoute>,
    pub debug: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOn {
    Join,
    Ping,
    Always,
}

#[derive(Clone)]
pub struct EnderRoute {
    pub protocol: Arc<dyn RefractiumProtocol>,
    pub targets: Vec<String>,
    pub startup_on: StartupOn,
    pub handler: Option<Arc<dyn LifecycleHandler>>,
    pub proxy: Option<Arc<dyn ServerProxy>>,
    pub shutdown_timeout_secs: u64,
    pub check_interval_secs: u64,
    pub min_players: usize,
    pub startup_webhook: Option<String>,
    pub shutdown_webhook: Option<String>,
    pub offline_motd: Option<String>,
    pub offline_message: Option<String>,
}

impl EnderRoute {
    #[must_use]
    pub fn new(protocol: Arc<dyn RefractiumProtocol>, targets: Vec<String>) -> Self {
        Self {
            protocol,
            targets,
            startup_on: StartupOn::Join,
            handler: None,
            proxy: None,
            shutdown_timeout_secs: 300,
            check_interval_secs: 60,
            min_players: 0,
            startup_webhook: None,
            shutdown_webhook: None,
            offline_motd: None,
            offline_message: None,
        }
    }
}

impl Default for EnderConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 25565,
            peek_buffer_size: 1024,
            peek_timeout_ms: 3000,
            upstreams: Vec::new(),
            debug: false,
        }
    }
}
