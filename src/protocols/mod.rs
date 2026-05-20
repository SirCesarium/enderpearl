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

use std::sync::Arc;

impl ProtocolKind {
    /// Creates a protocol instance for this kind, if the feature is enabled.
    #[must_use]
    pub fn instantiate(self) -> Option<Arc<dyn refractium::RefractiumProtocol>> {
        match self {
            Self::Java => {
                #[cfg(feature = "java")]
                {
                    Some(Arc::new(java::MinecraftJava))
                }
                #[cfg(not(feature = "java"))]
                {
                    None
                }
            }

            Self::Web => {
                #[cfg(feature = "web")]
                {
                    Some(Arc::new(web::HookedHttp::new()))
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
