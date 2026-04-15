pub mod registry;
pub mod routes;

use refractium::Refractium;
use std::{error, net::SocketAddr};

pub struct EnderRouter {
    inner: Refractium,
}

impl EnderRouter {
    pub fn new() -> Self {
        let (tcp_registry, udp_registry) = registry::create_registries();
        let config = routes::load_routes();

        let inner = Refractium::builder()
            .registries(tcp_registry, udp_registry)
            .routes(config.tcp, config.udp)
            .build();

        Self { inner }
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn error::Error>> {
        let t = self.inner.run_tcp(addr);
        let u = self.inner.run_udp(addr);

        println!("Enderpearl running on {addr} (TCP/UDP)");
        tokio::try_join!(t, u)?;
        Ok(())
    }
}
