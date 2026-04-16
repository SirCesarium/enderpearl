#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

use crate::display::EnderDisplay;
use anyhow::Context;
use enderpearl::core::router::EnderRouter;
use enderpearl::core::types::EnderConfig;
use std::net::SocketAddr;
use std::{fs, process};

mod config;
mod display;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            eprintln!("\n{} {}", " critical ".black().on_red().bold(), err);

            let mut current = err.source();
            while let Some(cause) = current {
                eprintln!("  {} {}", "└─>".dimmed(), cause.to_string().bright_yellow());
                current = cause.source();
            }
        }

        #[cfg(not(feature = "pretty-cli"))]
        eprintln!("CRITICAL ERROR: {:?}", err);

        process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    #[cfg(feature = "logging")]
    {
        use tracing_subscriber::{EnvFilter, fmt, prelude::*};

        tracing_subscriber::registry()
            .with(fmt::layer().with_target(false))
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
            .init();
    }

    let config_path = "enderpearl.toml";

    let config_str =
        fs::read_to_string(config_path).context("Could not find or read 'enderpearl.toml'")?;

    let toml_config: config::TomlConfig =
        toml::from_str(&config_str).context("The configuration file has invalid TOML syntax")?;

    let config = EnderConfig::from(toml_config);

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .with_context(|| {
            format!(
                "'{}' is not a valid address for binding",
                format_args!("{}:{}", config.bind, config.port)
            )
        })?;

    EnderDisplay::print_banner();
    EnderDisplay::print_listen(&addr);
    EnderDisplay::print_features();

    let router = EnderRouter::new(&config)
        .context("Router initialization failed (check if protocol features are enabled)")?;

    router
        .serve(addr)
        .await
        .context("The core engine stopped unexpectedly")?;

    Ok(())
}
