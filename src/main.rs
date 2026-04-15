#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod hooks;
mod protocols;
mod router;

use crate::router::EnderRouter;
use std::{error, net::SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let addr: SocketAddr = "0.0.0.0:25565".parse()?;

    let router = EnderRouter::new();
    router.serve(addr).await?;

    Ok(())
}
