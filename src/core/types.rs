use std::collections::HashMap;

pub struct EnderConfig {
    pub bind: String,
    pub port: u16,
    pub hot_reload: bool,
    pub peek_buffer_size: usize,
    pub peek_timeout_ms: u64,
    pub upstreams: HashMap<String, EnderRoute>,
}

pub struct EnderRoute {
    pub targets: Vec<String>,
    pub labels: Vec<String>,
    pub wake_command: Option<String>,
}

impl Default for EnderConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 25565,
            hot_reload: true,
            peek_buffer_size: 1024,
            peek_timeout_ms: 3000,
            upstreams: HashMap::new(),
        }
    }
}
