use enderpearl::core::types::{EnderConfig, EnderRoute};
use enderpearl::errors::{EnderError, Result};
use enderpearl::fail_config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "bedrock")]
use enderpearl::protocols::bedrock::MinecraftBedrock;
#[cfg(feature = "java")]
use enderpearl::protocols::java::MinecraftJava;
#[cfg(feature = "web")]
use enderpearl::protocols::web::HookedHttp;

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
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
    #[serde(default = "default_peek_buffer")]
    pub peek_buffer_size: usize,
    #[serde(default = "default_peek_timeout")]
    pub peek_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TomlRoute {
    pub forward_to: TomlTarget,
    pub labels: Option<Vec<String>>,
    pub wake_command: Option<String>,
    pub fake_motd: Option<String>,
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

                let protocol: Arc<dyn refractium::RefractiumProtocol> = match name.as_str() {
                    // Java
                    #[cfg(feature = "java")]
                    "minecraft_java" | "java" | "mcj" => Arc::new(MinecraftJava),
                    #[cfg(not(feature = "java"))]
                    "minecraft_java" | "java" | "mcj" => {
                        return fail_config!(name, "feature 'java' is disabled");
                    }

                    // Bedrock
                    #[cfg(feature = "bedrock")]
                    "minecraft_bedrock" | "bedrock" | "mcb" => Arc::new(MinecraftBedrock),
                    #[cfg(not(feature = "bedrock"))]
                    "minecraft_bedrock" | "bedrock" | "mcb" => {
                        return fail_config!(
                            name,
                            format!("feature '{}' is disabled", "bedrock".bright_red().bold())
                        );
                    }

                    // Web
                    #[cfg(feature = "web")]
                    "http" | "web" => Arc::new(HookedHttp::new()),
                    #[cfg(not(feature = "web"))]
                    "http" | "web" => {
                        return fail_config!(
                            name,
                            format!("feature '{}' is disabled", "web".bright_red().bold())
                        );
                    }

                    // Default
                    _ => return fail_config!(name, "unknown protocol".to_string()),
                };

                Ok(EnderRoute {
                    protocol,
                    targets,
                    wake_command: route.wake_command,
                    fake_motd: route.fake_motd,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            bind: toml.server.bind,
            port: toml.server.port,
            hot_reload: toml.server.hot_reload,
            peek_buffer_size: toml.server.peek_buffer_size,
            peek_timeout_ms: toml.server.peek_timeout_ms,
            upstreams,
            java_proxy_port: None,
        })
    }
}
