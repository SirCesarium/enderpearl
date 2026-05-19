use crate::EnderError;
use crate::core::{routes, types::EnderConfig};
use crate::errors::Result;
use refractium::Refractium;
use std::convert;
use std::net::SocketAddr;

pub struct EnderRouter {
    inner: Refractium,
}

impl EnderRouter {
    /// Creates a new `EnderRouter` from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if route mapping fails or the `Refractium` engine cannot be built.
    pub fn new(config: &EnderConfig) -> Result<Self> {
        let (tcp_routes, udp_routes) = routes::map_to_refractium(config)?;

        let inner = Refractium::builder()
            .routes(tcp_routes, udp_routes)
            .peek_config(config.peek_buffer_size, config.peek_timeout_ms)
            .build()
            .map_err(EnderError::Refractium)?;

        Ok(Self { inner })
    }

    /// Starts the proxy on the given address (TCP + UDP).
    ///
    /// # Errors
    ///
    /// Returns an error if the server cannot bind or encounters a fatal runtime error.
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        self.inner.run_both(addr).await.map_err(convert::Into::into)
    }
}
