use clap::{Parser, ValueEnum};
use enderpearl::{Config, WakeupCondition, run};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::process::Command;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum WakeupArg {
    Motd,
    Join,
    Disabled,
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "0.0.0.0:25565")]
    listen: String,
    #[arg(short, long)]
    web: Option<String>,
    #[arg(short, long, default_value = "127.0.0.1:25567")]
    mc: String,
    #[arg(short, long)]
    on_wakeup: Option<String>,
    #[arg(long, value_enum, default_value = "motd")]
    wakeup_on: WakeupArg,
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[arg(
        long,
        default_value = "§c§l⚡ §eServer currently waking up...\n§7Please wait a moment."
    )]
    msg_motd: String,

    #[arg(
        long,
        default_value = "§6§l⚡ §eServer still starting...\n\n§7Please wait a moment while the world loads.\n\n§8[§eNote§8] §eIf the ping bar stays §9blue/idle§e, please\n§etry to re-join manually in §c2 minutes§e."
    )]
    msg_starting: String,

    #[arg(
        long,
        default_value = "§6§l⚡ §eServer still starting...\n\n§c§lNext attempt will put you in a waitlist.\n§7(We will notify you when the server is ready)"
    )]
    msg_waitlist: String,

    #[arg(
        long,
        default_value = "§6Server §a§lONLINE§r§6!\n\n§6§lTry to join the server normally."
    )]
    msg_online: String,

    #[arg(
        long,
        default_value = "§c§l⚡ §eWaitlist timeout...\n\n§7The server is taking too long to start.\n§ePlease try again in a few minutes."
    )]
    msg_timeout: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let cond = match args.wakeup_on {
        WakeupArg::Motd => WakeupCondition::Motd,
        WakeupArg::Join => WakeupCondition::Join,
        WakeupArg::Disabled => WakeupCondition::Disabled,
    };

    let callback = args.on_wakeup.map(|cmd_str| {
        let debug = args.debug;
        let cb: enderpearl::WakeupCallback = Arc::new(move || {
            let cmd_to_run = cmd_str.clone();
            Box::pin(async move {
                if debug {
                    println!("Executing wakeup command...");
                }
                let mut cmd = if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd");
                    c.args(["/C", &cmd_to_run]);
                    c
                } else {
                    let mut c = Command::new("sh");
                    c.args(["-c", &cmd_to_run]);
                    c
                };
                cmd.kill_on_drop(true);
                match cmd.status().await {
                    Ok(s) if s.success() => {
                        if debug {
                            println!("Wakeup command executed successfully");
                        }
                    }
                    Ok(s) => eprintln!("Wakeup command failed: {}", s),
                    Err(e) => eprintln!("Failed to execute: {}", e),
                }
            })
        });
        cb
    });

    let cfg = Arc::new(Config {
        listen: args.listen,
        web: args.web,
        mc: args.mc,
        wakeup_on: cond,
        debug: args.debug,
        on_wakeup: callback,
        is_waking: AtomicBool::new(false),

        msg_motd: args.msg_motd,
        msg_starting: args.msg_starting,
        msg_waitlist: args.msg_waitlist,
        msg_online: args.msg_online,
        msg_timeout: args.msg_timeout,
    });

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        std::process::exit(0);
    });

    run(cfg).await
}
