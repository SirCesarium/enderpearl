use crate::core::{registry, routes, types::EnderConfig};
use crate::errors::Result;
use refractium::Refractium;
use std::net::SocketAddr;

pub struct EnderRouter {
    inner: Refractium,
}

impl EnderRouter {
    pub fn new(config: &EnderConfig) -> Result<Self> {
        let (tcp_registry, udp_registry) = registry::create_registries();
        let (tcp_routes, udp_routes) = routes::map_to_refractium(config)?;

        let inner = Refractium::builder()
            .registries(tcp_registry, udp_registry)
            .routes(tcp_routes, udp_routes)
            .build();

        Ok(Self { inner })
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let t = self.inner.run_tcp(addr);
        let u = self.inner.run_udp(addr);
        tokio::try_join!(t, u)?;
        Ok(())
    }
}
