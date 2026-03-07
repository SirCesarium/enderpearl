pub mod mc;

use dashmap::DashMap;
use futures::future::BoxFuture;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use t_port::{Protocol, identify, tunnel};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

pub type WakeupCallback = Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WakeupCondition {
    Motd,
    Join,
    Disabled,
}

pub struct Config {
    pub listen: String,
    pub web: Option<String>,
    pub mc: String,
    pub wakeup_on: WakeupCondition,
    pub debug: bool,
    pub on_wakeup: Option<WakeupCallback>,
    pub is_waking: AtomicBool,

    pub msg_motd: String,
    pub msg_starting: String,
    pub msg_waitlist: String,
    pub msg_online: String,
    pub msg_timeout: String,
}

pub struct UserHistory {
    pub attempts: u32,
    pub last_seen: Instant,
}

pub async fn run(cfg: Arc<Config>) -> Result<(), Box<dyn std::error::Error>> {
    let history: Arc<DashMap<String, UserHistory>> = Arc::new(DashMap::new());
    let listener = TcpListener::bind(&cfg.listen).await?;
    println!("Proxy active on {}", cfg.listen);

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
        Protocol::Http => {
            if let Some(web_target) = &cfg.web {
                tunnel(socket, web_target.clone()).await
            } else {
                if cfg.debug {
                    println!("HTTP request received but no web target configured. Closing.");
                }
                Ok(())
            }
        }
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
