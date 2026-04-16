use refractium::ProtocolRegistry;
use std::sync::Arc;

#[cfg(feature = "java")]
use crate::protocols::java::MinecraftJava;

#[cfg(feature = "bedrock")]
use crate::protocols::bedrock::MinecraftBedrock;

#[cfg(feature = "web")]
use crate::protocols::web::HookedHttp;

#[must_use]
pub fn create_registries() -> (Arc<ProtocolRegistry>, Arc<ProtocolRegistry>) {
    #[allow(unused_mut)]
    let mut tcp_r = ProtocolRegistry::new();
    #[allow(unused_mut)]
    let mut udp_r = ProtocolRegistry::new();

    #[cfg(feature = "java")]
    tcp_r.register(Arc::new(MinecraftJava));

    #[cfg(feature = "web")]
    tcp_r.register(Arc::new(HookedHttp::new()));

    #[cfg(feature = "bedrock")]
    udp_r.register(Arc::new(MinecraftBedrock));

    (Arc::new(tcp_r), Arc::new(udp_r))
}
