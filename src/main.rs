mod protocols;

use protocols::{MinecraftBedrock, MinecraftJava, UdpCommands};
use refractium::{Http, ProtocolRegistry, Refractium};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TCP Registry
    let mut tcp_r = ProtocolRegistry::new();
    tcp_r.register(Box::new(MinecraftJava));
    tcp_r.register(Box::new(Http));

    // UDP Registry
    let mut udp_r = ProtocolRegistry::new();
    udp_r.register(Box::new(MinecraftBedrock));
    udp_r.register(Box::new(UdpCommands));

    let refractium = Refractium::builder()
        .registries(Arc::new(tcp_r), Arc::new(udp_r))
        .build();

    let addr = "0.0.0.0:25565".parse()?;
    println!("Listening on {addr} (TCP/UDP)");

    let t = refractium.run_tcp(addr);
    let u = refractium.run_udp(addr);

    tokio::try_join!(t, u)?;

    Ok(())
}
