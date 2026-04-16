#![allow(dead_code)]

use enderpearl::core::types::{EnderConfig, EnderRoute};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TomlConfig {
    pub server: ServerConfig,
    pub upstream: HashMap<String, TomlRoute>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    pub port: u16,
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
    #[serde(default = "default_peek_buffer")]
    pub peek_buffer_size: usize,
    #[serde(default = "default_peek_timeout")]
    pub peek_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TomlRoute {
    pub forward_to: TomlTarget,
    pub labels: Option<Vec<String>>,
    pub wake_command: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum TomlTarget {
    Address(String),
    Pool(Vec<String>),
}

fn default_bind() -> String {
    "0.0.0.0".to_string()
}

const fn default_peek_buffer() -> usize {
    1024
}

const fn default_peek_timeout() -> u64 {
    3000
}

const fn default_hot_reload() -> bool {
    true
}

impl From<TomlConfig> for EnderConfig {
    fn from(toml: TomlConfig) -> Self {
        let upstreams = toml
            .upstream
            .into_iter()
            .map(|(name, route)| {
                let targets = match route.forward_to {
                    TomlTarget::Address(s) => vec![s],
                    TomlTarget::Pool(v) => v,
                };
                (
                    name,
                    EnderRoute {
                        targets,
                        labels: route.labels.unwrap_or_default(),
                        wake_command: route.wake_command,
                    },
                )
            })
            .collect();

        Self {
            bind: toml.server.bind,
            port: toml.server.port,
            hot_reload: toml.server.hot_reload,
            peek_buffer_size: toml.server.peek_buffer_size,
            peek_timeout_ms: toml.server.peek_timeout_ms,
            upstreams,
        }
    }
}

pub fn example_config() -> String {
    r#"
[server]
port = 25565

[upstream.minecraft_java]
forward_to = "127.0.0.1:25566"
labels = ["survival"]
wake_command = "docker start mc_server"

[upstream.web_static]
forward_to = "127.0.0.1:8080"
"#
    .to_string()
}
