#![deny(clippy::all)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::absolute_paths)]
#![allow(missing_docs, clippy::missing_errors_doc)]

use crate::display::EnderDisplay;
use anyhow::Context;
use clap::CommandFactory;
use clap::Parser;
use enderpearl::core::router::EnderRouter;
use enderpearl::core::types::EnderConfig;
use enderpearl::minecraft;
use enderpearl::protocols::{ProtocolKind, PROTOCOLS};
use std::net::SocketAddr;
use std::sync::Arc;
use std::{fs, process};

mod cli;
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

    let cli = cli::Cli::parse();

    match cli.command {
        Some(cli::Commands::Init) => {
            cli::handle_init(&cli.config)?;
            return Ok(());
        }
        Some(cli::Commands::Run) | None => {}
    }

    let Ok(config_str) = fs::read_to_string(&cli.config) else {
        if cli.command.is_none() {
            cli::Cli::command().print_help()?;
            eprintln!();
        } else {
            eprintln!("No config file found at '{}'", cli.config.display());
            eprintln!("Run `enderpearl init` to create one.");
        }
        return Ok(());
    };

    let toml_config: config::TomlConfig =
        toml::from_str(&config_str).context("The configuration file has invalid TOML syntax")?;

    let mut config = EnderConfig::try_from(toml_config)?;

    if let Some(route) = config
        .upstreams
        .iter()
        .find(|r| {
            let name = r.protocol.name();
            let is_java = PROTOCOLS.iter().any(|p| {
                matches!(p.kind, ProtocolKind::Java)
                    && (p.id == name || p.aliases.contains(&name.as_str()))
            });
            is_java && (r.fake_motd.is_some() || r.wake_command.is_some())
        })
    {
        let proxy = Arc::new(minecraft::java::JavaProxy {
            targets: route.targets.clone(),
            wake_command: route.wake_command.clone(),
            fake_motd: route.fake_motd.clone(),
        });
        config.java_proxy_port = Some(proxy.serve().await?);
    }

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
