use anyhow::Context;
use clap::{CommandFactory, Parser};
use enderpearl::cli::{Cli, Commands};
use enderpearl::config::TomlConfig;
use enderpearl::core::router::EnderRouter;
use enderpearl::core::types::EnderConfig;
use enderpearl::display::EnderDisplay;
use enderpearl::minecraft;
use enderpearl::protocols::{ProtocolKind, PROTOCOLS};
use std::net::SocketAddr;
use std::sync::Arc;
use std::{fs, process};

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

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            enderpearl::cli::handle_init(&cli.config)?;
            return Ok(());
        }
        Some(Commands::Run) | None => {}
    }

    let Ok(config_str) = fs::read_to_string(&cli.config) else {
        if cli.command.is_none() {
            Cli::command().print_help()?;
            eprintln!();
        } else {
            eprintln!("No config file found at '{}'", cli.config.display());
            eprintln!("Run `enderpearl init` to create one.");
        }
        return Ok(());
    };

    let toml_config: TomlConfig =
        toml::from_str(&config_str).context("The configuration file has invalid TOML syntax")?;

    let mut config = EnderConfig::try_from(toml_config)?;

    if let Some(route) = config
        .upstreams
        .iter()
        .find(|r| {
            let name = r.protocol.name();
            PROTOCOLS.iter().any(|p| {
                matches!(p.kind, ProtocolKind::Java)
                    && (p.id == name || p.aliases.contains(&name.as_str()))
            })
        })
    {
        let proxy = Arc::new(minecraft::java::JavaProxy {
            targets: route.targets.clone(),
            startup_cmd: route.startup_cmd.clone(),
            startup_on: route.startup_on,
            offline_motd: route.offline_motd.clone(),
            offline_message: route.offline_message.clone(),
            startup_webhook: route.startup_webhook.clone(),
            shutdown_webhook: route.shutdown_webhook.clone(),
            debug: config.debug,
            is_waking: std::sync::atomic::AtomicBool::new(false),
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

    for route in config.upstreams.clone() {
        if route.shutdown_cmd.is_some() {
            spawn_shutdown_monitor(route);
        }
    }

    let router = EnderRouter::new(&config)
        .context("Router initialization failed (check if protocol features are enabled)")?;

    router
        .serve(addr)
        .await
        .context("The core engine stopped unexpectedly")?;

    Ok(())
}

fn spawn_shutdown_monitor(route: enderpearl::core::types::EnderRoute) {
    let target = route.targets[0].clone();
    let cmd = route.shutdown_cmd.unwrap();
    let timeout_secs = route.shutdown_timeout_secs;
    let interval_secs = route.check_interval_secs;
    let min_players = route.min_players;
    let shutdown_webhook = route.shutdown_webhook;

    tokio::spawn(async move {
        let mut empty_since: Option<tokio::time::Instant> = None;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;

            match crate::minecraft::java::get_player_count(&target).await {
                Ok(count) if count <= min_players => {
                    let now = tokio::time::Instant::now();
                    match empty_since {
                        Some(start) => {
                            let elapsed = now.duration_since(start).as_secs();
                            if elapsed >= timeout_secs {
                                tracing::info!("Server {} has been below threshold ({} players) for {}s. Triggering final check...", target, count, elapsed);
                                // Final double-check before shutdown
                                match crate::minecraft::java::get_player_count(&target).await {
                                    Ok(final_count) if final_count <= min_players => {
                                        if let Err(e) = crate::minecraft::java::execute_command(&cmd, true) {
                                            tracing::error!("Auto-shutdown failed for {}: {}", target, e);
                                        } else {
                                            tracing::info!("Shutdown triggered for {}: {}", target, cmd);
                                            if let Some(ref url) = shutdown_webhook {
                                                let _ = crate::minecraft::java::send_webhook(url, &format!("Server {} shut down due to inactivity (players: {final_count})", target));
                                            }
                                        }
                                        empty_since = None; // Reset
                                    }
                                    Ok(final_count) => {
                                        tracing::info!("Shutdown aborted for {}: activity detected in final check ({} players)", target, final_count);
                                        empty_since = None;
                                    }
                                    Err(e) => {
                                        tracing::debug!("Final check for {} failed, assuming already offline: {}", target, e);
                                        empty_since = None;
                                    }
                                }
                            } else {
                                tracing::debug!("Server {} empty for {}/{}s", target, elapsed, timeout_secs);
                            }
                        }
                        None => {
                            tracing::info!("Server {} is now below threshold ({} players). Starting {}s shutdown timer.", target, count, timeout_secs);
                            empty_since = Some(now);
                        }
                    }
                }
                Ok(count) => {
                    if empty_since.is_some() {
                        tracing::info!("Activity detected on {} ({} players). Resetting shutdown timer.", target, count);
                    }
                    empty_since = None;
                }
                Err(e) => {
                    // Don't reset the timer on error (e.g. server starting up or transient network issue)
                    // unless we are sure the server is unreachable for a good reason.
                    tracing::trace!("Monitor check for {} failed: {}", target, e);
                }
            }
        }
    });
}

