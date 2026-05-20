use std::sync::Arc;

use refractium::RefractiumProtocol;

pub struct EnderConfig {
    pub bind: String,
    pub port: u16,
    pub peek_buffer_size: usize,
    pub peek_timeout_ms: u64,
    pub upstreams: Vec<EnderRoute>,
    pub java_proxy_port: Option<u16>,
    pub debug: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOn {
    Join,
    Ping,
    Always,
}

pub struct EnderRoute {
    pub protocol: Arc<dyn RefractiumProtocol>,
    pub targets: Vec<String>,
    pub startup_cmd: Option<String>,
    pub startup_on: StartupOn,
    pub offline_motd: Option<String>,
    pub offline_message: Option<String>,
}

impl Default for EnderConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 25565,
            peek_buffer_size: 1024,
            peek_timeout_ms: 3000,
            upstreams: Vec::new(),
            java_proxy_port: None,
            debug: false,
        }
    }
}
