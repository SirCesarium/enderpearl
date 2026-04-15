mod hooks;
mod protocols;
mod router;

use crate::router::EnderRouter;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "0.0.0.0:25565".parse()?;

    let router = EnderRouter::new();
    router.serve(addr).await?;

    Ok(())
}
