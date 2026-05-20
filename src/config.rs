use crate::core::types::{EnderConfig, EnderRoute, StartupOn};
use crate::errors::{EnderError, Result};
use crate::protocols::ProtocolMeta;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TomlConfig {
    pub server: ServerConfig,
    pub upstream: HashMap<String, TomlRoute>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    pub port: u16,
    #[serde(default = "default_peek_buffer")]
    pub peek_buffer_size: usize,
    #[serde(default = "default_peek_timeout")]
    pub peek_timeout_ms: u64,
    #[serde(default)]
    pub debug: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TomlRoute {
    pub forward_to: TomlTarget,
    #[serde(alias = "wake_command")]
    pub startup_cmd: Option<String>,
    #[serde(default = "default_startup_on")]
    pub startup_on: TomlStartupOn,
    pub shutdown_cmd: Option<String>,
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64,
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default)]
    pub min_players: usize,
    pub startup_webhook: Option<String>,
    pub shutdown_webhook: Option<String>,
    pub offline_motd: Option<String>,
    pub offline_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TomlStartupOn {
    Join,
    Ping,
    Always,
}

fn default_startup_on() -> TomlStartupOn {
    TomlStartupOn::Join
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

const fn default_shutdown_timeout() -> u64 {
    300
}

const fn default_check_interval() -> u64 {
    60
}

impl TryFrom<TomlConfig> for EnderConfig {
    type Error = EnderError;

    fn try_from(toml: TomlConfig) -> Result<Self> {
        let upstreams = toml
            .upstream
            .into_iter()
            .map(|(name, route)| {
                let targets = match route.forward_to {
                    TomlTarget::Address(s) => vec![s],
                    TomlTarget::Pool(v) => v,
                };

                let meta = ProtocolMeta::lookup(&name).ok_or_else(|| {
                    EnderError::Config(name.clone(), "unknown protocol".to_string())
                })?;

                let protocol = meta.kind.instantiate(toml.server.debug).ok_or_else(|| {
                    EnderError::Config(name.clone(), format!("requires '{}' feature", meta.feature))
                })?;

                Ok(EnderRoute {
                    protocol,
                    targets,
                    startup_on: match route.startup_on {
                        TomlStartupOn::Join => StartupOn::Join,
                        TomlStartupOn::Ping => StartupOn::Ping,
                        TomlStartupOn::Always => StartupOn::Always,
                    },
                    handler: None, // Will be injected by the caller (CLI or Lib user)
                    shutdown_timeout_secs: route.shutdown_timeout,
                    check_interval_secs: route.check_interval,
                    min_players: route.min_players,
                    startup_webhook: route.startup_webhook,
                    shutdown_webhook: route.shutdown_webhook,
                    offline_motd: route.offline_motd,
                    offline_message: route.offline_message,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            bind: toml.server.bind,
            port: toml.server.port,
            peek_buffer_size: toml.server.peek_buffer_size,
            peek_timeout_ms: toml.server.peek_timeout_ms,
            upstreams,
            java_proxy_port: None,
            debug: toml.server.debug,
        })
    }
}
