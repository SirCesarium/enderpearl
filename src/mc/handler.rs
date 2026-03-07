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
            Self::handle_wakeup(socket, cfg).await
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
                Self::handle_waitlist(socket, cfg).await
            } else {
                Self::handle_disconnect(socket, attempts, cfg).await
            }
        }
    }

    async fn handle_wakeup(socket: &mut TcpStream, cfg: Arc<Config>) -> tokio::io::Result<()> {
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
                text: cfg.msg_motd.clone(),
            },
        };

        MinecraftPacket::send_json(socket, 0x00, &response).await?;

        let mut buf = [0u8; 32];
        if let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_secs(2), socket.read(&mut buf[..])).await
        {
            let start = Instant::now();
            while start.elapsed().as_secs() < 120 {
                if TcpStream::connect(&cfg.mc).await.is_ok() {
                    tokio::io::AsyncWriteExt::write_all(socket, &buf[..n]).await?;
                    return Ok(());
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }

    async fn handle_waitlist(socket: &mut TcpStream, cfg: Arc<Config>) -> tokio::io::Result<()> {
        let start = Instant::now();
        while start.elapsed().as_secs() < 28 {
            if TcpStream::connect(&cfg.mc).await.is_ok() {
                let res = DisconnectResponse {
                    text: cfg.msg_online.clone(),
                };
                return MinecraftPacket::send_json(socket, 0x00, &res).await;
            }
            sleep(Duration::from_millis(800)).await;
        }

        let res = DisconnectResponse {
            text: cfg.msg_timeout.clone(),
        };
        MinecraftPacket::send_json(socket, 0x00, &res).await
    }

    async fn handle_disconnect(socket: &mut TcpStream, attempts: u32, cfg: Arc<Config>) -> tokio::io::Result<()> {
        sleep(Duration::from_millis(10)).await;
        let text = if attempts == 1 {
            &cfg.msg_starting
        } else {
            &cfg.msg_waitlist
        }.to_string();

        MinecraftPacket::send_json(socket, 0x00, &DisconnectResponse { text }).await
    }

    async fn trigger_wakeup(cfg: Arc<Config>) {
        if let Some(callback) = &cfg.on_wakeup {
            if cfg
                .is_waking
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return;
            }

            let cb = Arc::clone(callback);
            tokio::spawn(async move {
                cb().await;
            });
        }
    }
}
