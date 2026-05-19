use std::sync::Arc;

use refractium::RefractiumProtocol;

pub struct EnderConfig {
    pub bind: String,
    pub port: u16,
    pub peek_buffer_size: usize,
    pub peek_timeout_ms: u64,
    pub upstreams: Vec<EnderRoute>,
    pub java_proxy_port: Option<u16>,
}

pub struct EnderRoute {
    pub protocol: Arc<dyn RefractiumProtocol>,
    pub targets: Vec<String>,
    pub wake_command: Option<String>,
    pub fake_motd: Option<String>,
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
        }
    }
}
