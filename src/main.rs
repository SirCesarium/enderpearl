#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod errors;
mod hooks;
mod protocols;
mod router;

use crate::router::EnderRouter;
use anyhow::Context;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = "0.0.0.0:25565"
        .parse()
        .context("Failed to parse listener address")?;

    let router = EnderRouter::new();

    router
        .serve(addr)
        .await
        .context("Enderpearl router stopped unexpectedly")?;

    Ok(())
}
