use clap::{Parser, ValueEnum};
use mc_gate::{Config, WakeupCondition, run};
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
    #[arg(short, long, default_value = "127.0.0.1:80")]
    web: String,
    #[arg(short, long, default_value = "127.0.0.1:25567")]
    mc: String,
    #[arg(short, long)]
    on_wakeup: Option<String>,
    #[arg(long, value_enum, default_value = "motd")]
    wakeup_on: WakeupArg,
    #[arg(short, long, default_value_t = false)]
    debug: bool,
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
        let cb: mc_gate::WakeupCallback = Arc::new(move || {
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
    });

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        std::process::exit(0);
    });

    run(cfg).await
}
