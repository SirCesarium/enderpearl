mod mc;

use clap::{Parser, ValueEnum};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use t_port::{Protocol, identify, tunnel};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum WakeupCondition {
    Motd,
    Join,
    Disabled,
}

#[derive(Parser)]
struct Config {
    #[arg(short, long, default_value = "0.0.0.0:25565")]
    listen: String,
    #[arg(short, long, default_value = "127.0.0.1:80")]
    web: String,
    #[arg(short, long, default_value = "127.0.0.1:25567")]
    mc: String,
    #[arg(short, long)]
    on_wakeup: Option<String>,
    #[arg(long, value_enum, default_value = "motd")]
    wakeup_on: WakeupCondition,
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[arg(skip)]
    pub is_waking: AtomicBool,
}

pub struct UserHistory {
    pub attempts: u32,
    pub last_seen: Instant,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Arc::new(Config::parse());
    let history: Arc<DashMap<String, UserHistory>> = Arc::new(DashMap::new());
    let listener = TcpListener::bind(&cfg.listen).await?;
    println!("Proxy active on {}", cfg.listen);

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        std::process::exit(0);
    });

    let history_gc = Arc::clone(&history);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            history_gc.retain(|_, v| v.last_seen.elapsed() < Duration::from_secs(300));
        }
    });

    loop {
        let (socket, addr) = listener.accept().await?;
        let cfg = Arc::clone(&cfg);
        let history = Arc::clone(&history);
        let ip = addr.ip().to_string();

        tokio::spawn(async move {
            let _ = process(socket, cfg, history, ip).await;
        });
    }
}

async fn process(
    mut socket: TcpStream,
    cfg: Arc<Config>,
    history: Arc<DashMap<String, UserHistory>>,
    ip: String,
) -> tokio::io::Result<()> {
    let mut head = [0u8; 8];
    let n = timeout(Duration::from_secs(2), socket.peek(&mut head[..]))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "peek timeout"))??;

    match identify(&head[..n]) {
        Protocol::Http => tunnel(socket, cfg.web.clone()).await,
        Protocol::Binary => {
            if let Ok(Ok(mut target)) =
                timeout(Duration::from_secs(1), TcpStream::connect(&cfg.mc)).await
            {
                cfg.is_waking.store(false, Ordering::SeqCst);
                tokio::io::copy_bidirectional(&mut socket, &mut target).await?;
                return Ok(());
            }

            if cfg.on_wakeup.is_some() {
                let state = mc::inspect_handshake(&socket).await;
                mc::handler::McHandler::send_fallback(&mut socket, state, cfg, history, ip).await
            } else {
                if cfg.debug {
                    println!(
                        "Connection to {} failed and no on-wakeup command set. Closing.",
                        cfg.mc
                    );
                }
                Ok(())
            }
        }
    }
}
