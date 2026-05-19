use crate::config::TomlTarget;
use crate::display::EnderDisplay;
use clap::{Parser, Subcommand};
use inquire::validator::Validation;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "enderpearl",
    version,
    about = "Async proxy for Minecraft and HTTP traffic",
    args_conflicts_with_subcommands = false
)]
pub struct Cli {
    #[arg(short, long, default_value = "enderpearl.toml", global = true)]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Run,
    Init,
}

// ── Init command ──

use inquire::{Confirm, CustomType, MultiSelect, Text};

/// Runs the interactive configuration wizard.
///
/// # Errors
///
/// Returns an error if user input cannot be collected or the config file cannot be written.
pub fn handle_init(config_path: &Path) -> anyhow::Result<()> {
    set_prompt_theme();

    println!();
    EnderDisplay::print_banner();
    println!();

    section_header("Enderpearl Configuration");

    let bind = Text::new("Bind address:")
        .with_default("0.0.0.0")
        .with_help_message("IP address for the proxy to listen on (0.0.0.0 = all interfaces)")
        .prompt()?;

    let port: u16 = CustomType::new("Port:")
        .with_default(25565)
        .with_help_message("Port for the proxy to listen on (default 25565 for Minecraft)")
        .with_error_message("Enter a valid port number (1–65535)")
        .prompt()?;

    section_header("Protocols");

    let selections = MultiSelect::new(
        "Select protocols to configure:",
        PROTOCOL_METAS.iter().map(|p| p.display_name).collect(),
    )
    .with_help_message("space to toggle, enter to confirm")
    .prompt()?;

    if selections.is_empty() {
        println!("  No protocols selected");
        println!();
        return Ok(());
    }

    let upstreams = collect_upstream_configs(&selections)?;

    if config_path.exists() {
        let overwrite = Confirm::new("File already exists. Overwrite?")
            .with_default(false)
            .prompt()?;
        if !overwrite {
            return Ok(());
        }
    }

    let config = InitConfig {
        server: ServerEntry { bind, port },
        upstream: upstreams,
    };

    let toml_str = toml::to_string_pretty(&config)?;
    fs::write(config_path, toml_str)?;

    section_header(format!("Written to {}", config_path.display()));

    Ok(())
}

// ── TOML serialization types ──

#[derive(Serialize)]
struct InitConfig {
    server: ServerEntry,
    upstream: HashMap<String, UpstreamConfig>,
}

#[derive(Serialize)]
struct ServerEntry {
    bind: String,
    port: u16,
}

#[derive(Serialize)]
struct UpstreamConfig {
    forward_to: TomlTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    wake_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fake_motd: Option<String>,
}

fn collect_upstream_configs(
    selections: &[&str],
) -> anyhow::Result<HashMap<String, UpstreamConfig>> {
    let mut upstreams = HashMap::new();

    for proto in selections {
        section_header(proto);

        let meta = PROTOCOL_METAS
            .iter()
            .find(|m| m.display_name == *proto)
            .ok_or_else(|| anyhow::anyhow!("unknown protocol '{proto}'"))?;

        let target = Text::new("Forward to:")
            .with_default(meta.default_port)
            .with_help_message("Server address — separate multiple with commas for round-robin")
            .with_validator(validate_address)
            .prompt()?;

        let wake = Text::new("Wake command (optional):")
            .with_help_message(
                "Shell command to start the server on traffic — e.g. docker start mc",
            )
            .prompt()?;
        let wake = if wake.trim().is_empty() {
            None
        } else {
            Some(wake.trim().to_string())
        };

        let fake_motd = Text::new("Fake MOTD JSON (optional):")
            .with_help_message(
                "Leave empty to disable. JSON with version, players, description fields",
            )
            .prompt()?;
        let fake_motd = if fake_motd.trim().is_empty() {
            None
        } else {
            Some(fake_motd.trim().to_string())
        };

        let forward_to = if target.contains(',') {
            TomlTarget::Pool(target.split(',').map(|s| s.trim().to_string()).collect())
        } else {
            TomlTarget::Address(target.trim().to_string())
        };

        upstreams.insert(
            meta.config_key.to_string(),
            UpstreamConfig {
                forward_to,
                wake_command: wake,
                fake_motd,
            },
        );
    }

    Ok(upstreams)
}

#[allow(clippy::unnecessary_wraps)]
fn validate_address(s: &str) -> Result<Validation, Box<dyn Error + Send + Sync>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Validation::Invalid(
            "Address is required — e.g. 127.0.0.1:25566".into(),
        ));
    }
    for addr in s.split(',') {
        let addr = addr.trim();
        if addr.is_empty() {
            return Ok(Validation::Invalid("Pool entry cannot be empty".into()));
        }
        if !addr.contains(':') {
            return Ok(Validation::Invalid(
                format!("'{addr}' needs a port — use format host:port").into(),
            ));
        }
        if let Some(port_str) = addr.rsplit(':').next()
            && port_str.parse::<u16>().is_err()
        {
            return Ok(Validation::Invalid(
                format!("'{port_str}' is not a valid port number (1–65535)").into(),
            ));
        }
    }
    Ok(Validation::Valid)
}

struct ProtocolMeta {
    display_name: &'static str,
    config_key: &'static str,
    default_port: &'static str,
}

const PROTOCOL_METAS: &[ProtocolMeta] = &[
    ProtocolMeta {
        display_name: "Minecraft Java",
        config_key: "minecraft_java",
        default_port: "127.0.0.1:25566",
    },
    ProtocolMeta {
        display_name: "Minecraft Bedrock",
        config_key: "minecraft_bedrock",
        default_port: "127.0.0.1:19132",
    },
    ProtocolMeta {
        display_name: "Web/HTTP",
        config_key: "web",
        default_port: "127.0.0.1:8080",
    },
];

// ── Prompt theme & rendering ──

fn set_prompt_theme() {
    use inquire::ui::{Attributes, Color, Styled};

    let prefix = Styled::new("▪ ")
        .with_fg(Color::LightMagenta)
        .with_attr(Attributes::BOLD);

    let check = Styled::new("✓ ").with_fg(Color::LightGreen);
    let cross = Styled::new("✗ ").with_fg(Color::DarkGrey);

    inquire::set_global_render_config(
        inquire::ui::RenderConfig::default_colored()
            .with_prompt_prefix(prefix)
            .with_answered_prompt_prefix(prefix)
            .with_selected_checkbox(check)
            .with_unselected_checkbox(cross),
    );
}

fn section_header(msg: impl Display) {
    println!();
    println!(
        " {} {}",
        "■".bright_magenta(),
        msg.to_string().bright_magenta().bold()
    );
    println!(" {}", "─".repeat(45).magenta());
}
