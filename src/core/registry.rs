#[cfg(feature = "web")]
use crate::protocols::HookedHttp;
use refractium::ProtocolRegistry;
use std::sync::Arc;

#[cfg(feature = "java")]
use crate::protocols::MinecraftJava;

#[cfg(feature = "bedrock")]
use crate::protocols::MinecraftBedrock;

#[must_use]
pub fn create_registries() -> (Arc<ProtocolRegistry>, Arc<ProtocolRegistry>) {
    let tcp_r = ProtocolRegistry::new();

    #[cfg(any(feature = "java", feature = "web"))]
    let mut tcp_r = tcp_r;

    let udp_r = ProtocolRegistry::new();

    #[cfg(feature = "bedrock")]
    let mut udp_r = udp_r;

    #[cfg(feature = "java")]
    tcp_r.register(Arc::new(MinecraftJava));

    #[cfg(feature = "web")]
    tcp_r.register(Arc::new(HookedHttp::new()));

    #[cfg(feature = "bedrock")]
    udp_r.register(Arc::new(MinecraftBedrock));

    (Arc::new(tcp_r), Arc::new(udp_r))
}
