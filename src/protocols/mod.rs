use std::fmt;

#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "java")]
pub mod java;
#[cfg(feature = "web")]
pub mod web;

#[derive(Debug, Clone, Copy)]
pub enum ProtocolKind {
    Java,
    Bedrock,
    Web,
}

impl fmt::Display for ProtocolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Java => "Minecraft Java",
            Self::Bedrock => "Minecraft Bedrock",
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
}

pub const PROTOCOLS: &[ProtocolMeta] = &[
    ProtocolMeta {
        kind: ProtocolKind::Java,
        id: "minecraftjava",
        aliases: &["minecraft_java", "java", "mcj"],
        feature: "java",
        is_enabled: cfg!(feature = "java"),
    },
    ProtocolMeta {
        kind: ProtocolKind::Bedrock,
        id: "minecraftbedrock",
        aliases: &["minecraft_bedrock", "bedrock", "mcb"],
        feature: "bedrock",
        is_enabled: cfg!(feature = "bedrock"),
    },
    ProtocolMeta {
        kind: ProtocolKind::Web,
        id: "http",
        aliases: &["web", "http", "website"],
        feature: "web",
        is_enabled: cfg!(feature = "web"),
    },
];
