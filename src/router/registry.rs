use crate::protocols::{HookedHttp, MinecraftBedrock, MinecraftJava};
use refractium::ProtocolRegistry;
use std::sync::Arc;

pub fn create_registries() -> (Arc<ProtocolRegistry>, Arc<ProtocolRegistry>) {
    // TCP/UDP REGISTRY
    let mut tcp_r = ProtocolRegistry::new();
    let mut udp_r = ProtocolRegistry::new();

    // TCP PROTOCOLS
    tcp_r.register(Arc::new(MinecraftJava));
    tcp_r.register(Arc::new(HookedHttp::new()));

    // UDP PROTOCOLS
    udp_r.register(Arc::new(MinecraftBedrock));

    // EXPORT PROTOCOLS
    (Arc::new(tcp_r), Arc::new(udp_r))
}
