#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

use anyhow::Context;
use enderpearl::EnderRouter;
use std::net::SocketAddr;

use crate::display::EnderDisplay;

mod display;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = "0.0.0.0:25565"
        .parse()
        .context("Failed to parse listener address")?;

    EnderDisplay::print_banner();
    EnderDisplay::print_listen(&addr);
    EnderDisplay::print_features();

    let router = EnderRouter::new();

    router
        .serve(addr)
        .await
        .context("Enderpearl core engine execution failed")?;

    Ok(())
}
