use std::fmt;

#[cfg(feature = "java")]
pub mod java;
#[cfg(feature = "web")]
pub mod web;

#[derive(Debug, Clone, Copy)]
pub enum ProtocolKind {
    Java,

    Web,
}

use crate::hooks::DebugHook;
use crate::protocols::java::MinecraftJava;
use refractium::hook_protocol;
#[cfg(feature = "web")]
use refractium::protocols::http::Http;
use std::sync::Arc;

#[cfg(feature = "java")]
hook_protocol!(
    wrapper: HookedJava,
    proto: MinecraftJava,
    hooks: [DebugHook]
);

impl ProtocolKind {
    /// Creates a protocol instance for this kind, if the feature is enabled.
    #[must_use]
    pub fn instantiate(self, debug: bool) -> Option<Arc<dyn refractium::RefractiumProtocol>> {
        match self {
            Self::Java => {
                #[cfg(feature = "java")]
                {
                    if debug {
                        Some(Arc::new(HookedJava::new()))
                    } else {
                        Some(Arc::new(java::MinecraftJava))
                    }
                }
                #[cfg(not(feature = "java"))]
                {
                    None
                }
            }

            Self::Web => {
                #[cfg(feature = "web")]
                {
                    if debug {
                        Some(Arc::new(web::HookedHttp::new()))
                    } else {
                        Some(Arc::new(Http))
                    }
                }
                #[cfg(not(feature = "web"))]
                {
                    None
                }
            }
        }
    }
}

impl fmt::Display for ProtocolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Java => "Minecraft Java",

            Self::Web => "Web/HTTP",
        };
        write!(f, "{name}")
    }
}

pub struct ProtocolMeta {
    pub kind: ProtocolKind,
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub feature: &'static str,
    pub is_enabled: bool,
    pub display_name: &'static str,
    pub config_key: &'static str,
    pub default_port: &'static str,
}

pub const PROTOCOLS: &[ProtocolMeta] = &[
    ProtocolMeta {
        kind: ProtocolKind::Java,
        id: "minecraftjava",
        aliases: &["minecraft_java", "java", "mcj"],
        feature: "java",
        is_enabled: cfg!(feature = "java"),
        display_name: "Minecraft Java",
        config_key: "minecraft_java",
        default_port: "127.0.0.1:25566",
    },
    ProtocolMeta {
        kind: ProtocolKind::Web,
        id: "http",
        aliases: &["web", "http", "website"],
        feature: "web",
        is_enabled: cfg!(feature = "web"),
        display_name: "Web/HTTP",
        config_key: "web",
        default_port: "127.0.0.1:8080",
    },
];

impl ProtocolMeta {
    #[must_use]
    pub fn lookup(name: &str) -> Option<&'static Self> {
        PROTOCOLS
            .iter()
            .find(|p| p.id == name || p.aliases.contains(&name))
    }
}
