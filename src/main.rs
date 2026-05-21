use anyhow::Context;
use clap::{CommandFactory, Parser};
use enderpearl::cli::{Cli, Commands};
use enderpearl::config::TomlConfig;
use enderpearl::core::router::EnderRouter;
use enderpearl::core::types::{EnderConfig, LifecycleHandler, AsyncResultFuture};
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

struct ShellLifecycleHandler {
    startup_cmd: Option<String>,
    shutdown_cmd: Option<String>,
}

impl LifecycleHandler for ShellLifecycleHandler {
    fn on_startup(&self) -> AsyncResultFuture {
        let cmd = self.startup_cmd.clone();
        Box::pin(async move {
            if let Some(c) = cmd {
                enderpearl::minecraft::java::execute_command(&c, false)?;
            }
            Ok(())
        })
    }

    fn on_shutdown(&self) -> AsyncResultFuture {
        let cmd = self.shutdown_cmd.clone();
        Box::pin(async move {
            if let Some(c) = cmd {
                enderpearl::minecraft::java::execute_command(&c, true)?;
            }
            Ok(())
        })
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

    let mut config = EnderConfig::try_from(toml_config.clone())?;

    // Inject shell handlers from TOML into the agnostic config
    for (name, toml_route) in &toml_config.upstream {
        if let Some(route) = config.upstreams.iter_mut().find(|r| r.protocol.name() == *name || name == "minecraft_java")
            && (toml_route.startup_cmd.is_some() || toml_route.shutdown_cmd.is_some())
        {
            route.handler = Some(Arc::new(ShellLifecycleHandler {
                startup_cmd: toml_route.startup_cmd.clone(),
                shutdown_cmd: toml_route.shutdown_cmd.clone(),
            }));
        }
    }

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
            startup_on: route.startup_on,
            handler: route.handler.clone(),
            shutdown_timeout_secs: route.shutdown_timeout_secs,
            check_interval_secs: route.check_interval_secs,
            min_players: route.min_players,
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

    let router = EnderRouter::new(&config)
        .context("Router initialization failed (check if protocol features are enabled)")?;

    router
        .serve(addr)
        .await
        .context("The core engine stopped unexpectedly")?;

    Ok(())
}
