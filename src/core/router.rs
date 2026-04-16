use crate::core::{registry, routes};
use crate::errors::Result;
use refractium::Refractium;
use std::net::SocketAddr;

pub struct EnderRouter {
    inner: Refractium,
}

impl Default for EnderRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl EnderRouter {
    #[must_use]
    pub fn new() -> Self {
        let (tcp_registry, udp_registry) = registry::create_registries();
        let config = routes::load_routes();
        let inner = Refractium::builder()
            .registries(tcp_registry, udp_registry)
            .routes(config.tcp, config.udp)
            .build();
        Self { inner }
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        let t = self.inner.run_tcp(addr);
        let u = self.inner.run_udp(addr);
        tokio::try_join!(t, u)?;
        Ok(())
    }
}
