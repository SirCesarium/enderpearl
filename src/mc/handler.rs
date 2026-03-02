use crate::mc::{
    Description, DisconnectResponse, MinecraftPacket, Players, StatusResponse, Version,
};
use crate::{Config, UserHistory, WakeupCondition};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::sleep;

pub struct McHandler;

impl McHandler {
    pub async fn send_fallback(
        socket: &mut TcpStream,
        state: i32,
        cfg: Arc<Config>,
        history: Arc<DashMap<String, UserHistory>>,
        ip: String,
    ) -> tokio::io::Result<()> {
        if cfg.on_wakeup.is_none() {
            return Ok(());
        }

        let should_trigger = match cfg.wakeup_on {
            WakeupCondition::Disabled => false,
            WakeupCondition::Motd => state == 1 || state == 2,
            WakeupCondition::Join => state == 2,
        };

        if should_trigger {
            Self::trigger_wakeup(Arc::clone(&cfg)).await;
        }

        if state == 1 {
            Self::handle_wakeup(socket, cfg.mc.clone()).await
        } else {
            let attempts = {
                let mut entry = history.entry(ip).or_insert(UserHistory {
                    attempts: 0,
                    last_seen: Instant::now(),
                });

                if entry.last_seen.elapsed() > Duration::from_secs(300) {
                    entry.attempts = 1;
                } else {
                    entry.attempts += 1;
                }
                entry.last_seen = Instant::now();
                entry.attempts
            };

            if attempts >= 3 {
                Self::handle_waitlist(socket, cfg.mc.clone()).await
            } else {
                Self::handle_disconnect(socket, attempts).await
            }
        }
    }

    async fn handle_wakeup(socket: &mut TcpStream, target: String) -> tokio::io::Result<()> {
        let response = StatusResponse {
            version: Version {
                name: "mc-gate".to_string(),
                protocol: 767,
            },
            players: Players {
                max: 0,
                online: "???".to_string(),
            },
            description: Description {
                text: "§c§l⚡ §eServer currently waking up...\n§7Please wait a moment.".to_string(),
            },
        };

        MinecraftPacket::send_json(socket, 0x00, &response).await?;

        let mut buf = [0u8; 32];
        if let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_secs(2), socket.read(&mut buf[..])).await
        {
            let start = Instant::now();
            while start.elapsed().as_secs() < 120 {
                if TcpStream::connect(&target).await.is_ok() {
                    tokio::io::AsyncWriteExt::write_all(socket, &buf[..n]).await?;
                    return Ok(());
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }

    async fn handle_waitlist(socket: &mut TcpStream, target: String) -> tokio::io::Result<()> {
        let start = Instant::now();
        while start.elapsed().as_secs() < 28 {
            if TcpStream::connect(&target).await.is_ok() {
                let res = DisconnectResponse {
                    text: "§6Server §a§lONLINE§r§6!\n\n§6§lTry to join the server normally."
                        .to_string(),
                };
                return MinecraftPacket::send_json(socket, 0x00, &res).await;
            }
            sleep(Duration::from_millis(800)).await;
        }

        let res = DisconnectResponse {
            text: "§c§l⚡ §eWaitlist timeout...\n\n§7The server is taking too long to start.\n§ePlease try again in a few minutes.".to_string(),
        };
        MinecraftPacket::send_json(socket, 0x00, &res).await
    }

    async fn handle_disconnect(socket: &mut TcpStream, attempts: u32) -> tokio::io::Result<()> {
        sleep(Duration::from_millis(10)).await;
        let text = if attempts == 1 {
            "§6§l⚡ §eServer still starting...\n\n§7Please wait a moment while the world loads.\n\n§8[§eNote§8] §eIf the ping bar stays §9blue/idle§e, please\n§etry to re-join manually in §c2 minutes§e.".to_string()
        } else {
            "§6§l⚡ §eServer still starting...\n\n§c§lNext attempt will put you in a waitlist.\n§7(We will notify you when the server is ready)".to_string()
        };

        MinecraftPacket::send_json(socket, 0x00, &DisconnectResponse { text }).await
    }

    async fn trigger_wakeup(cfg: Arc<Config>) {
        if let Some(raw_cmd) = &cfg.on_wakeup {
            if cfg
                .is_waking
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return;
            }

            let cmd_to_run = raw_cmd.clone();
            let cfg_clone = Arc::clone(&cfg);

            tokio::spawn(async move {
                if cfg_clone.debug {
                    println!("Executing wakeup command...");
                }

                let mut cmd = if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd");
                    c.args(["/C", &cmd_to_run]);
                    c
                } else {
                    let mut c = Command::new("sh");
                    c.args(["-c", &cmd_to_run]);
                    #[cfg(unix)]
                    c.process_group(0);
                    c
                };

                cmd.kill_on_drop(true);

                match cmd.status().await {
                    Ok(status) if status.success() => {
                        if cfg_clone.debug {
                            println!("Wakeup command executed successfully");
                        }
                    }
                    Ok(status) => {
                        eprintln!("Wakeup command failed: {}", status);
                        cfg_clone.is_waking.store(false, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!("Failed to execute: {}", e);
                        cfg_clone.is_waking.store(false, Ordering::SeqCst);
                    }
                }
            });
        }
    }
}
