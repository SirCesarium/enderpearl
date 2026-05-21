use crate::error;
use crate::core::types::{StartupOn, LifecycleHandler};
use crate::errors::{EnderError, Result};
use crate::minecraft::varint::{decode_string, decode_varint, encode_mc_packet, encode_varint, read_varint};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use serde_json::json;
use reqwest::Client;

const DEFAULT_MOTD: &str = r#"{"version":{"name":"1.21","protocol":766},"players":{"max":0,"online":0},"description":{"text":"§cServer offline — starting up..."}}"#;
const DEFAULT_DISCONNECT: &str = r#"{"text":"§cServer offline — starting up..."}"#;

pub struct Handshake {
    pub proto_ver: i32,
    pub addr: String,
    pub port: u16,
    pub next_state: i32,
    pub raw: Vec<u8>,
}

pub struct JavaProxy {
    pub targets: Vec<String>,
    pub startup_on: StartupOn,
    pub handler: Option<Arc<dyn LifecycleHandler>>,
    pub shutdown_timeout_secs: u64,
    pub check_interval_secs: u64,
    pub min_players: usize,
    pub offline_motd: Option<String>,
    pub offline_message: Option<String>,
    pub startup_webhook: Option<String>,
    pub shutdown_webhook: Option<String>,
    pub debug: bool,
    pub is_waking: AtomicBool,
}

impl JavaProxy {
    /// Binds a TCP listener on `127.0.0.1:0` and spawns the accept loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP listener cannot be bound.
    pub async fn serve(self: Arc<Self>) -> Result<u16> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();

        let proxy_accept = self.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let proxy = proxy_accept.clone();
                        tokio::spawn(async move {
                            let _ = proxy.handle_connection(stream).await;
                        });
                    }
                    Err(e) => {
                        error!("Java proxy accept error: {e}");
                    }
                }
            }
        });

        // Spawn inactivity monitor if a handler is present and timeout is configured
        if self.handler.is_some() && self.shutdown_timeout_secs > 0 {
            let monitor_proxy = self.clone();
            tokio::spawn(async move {
                monitor_proxy.spawn_monitor().await;
            });
        }

        Ok(port)
    }

    async fn spawn_monitor(&self) {
        let mut empty_since: Option<tokio::time::Instant> = None;
        let target = self.targets[0].clone();

        loop {
            tokio::time::sleep(Duration::from_secs(self.check_interval_secs)).await;

            match get_player_count(&target).await {
                Ok(count) if count <= self.min_players => {
                    let now = tokio::time::Instant::now();
                    match empty_since {
                        Some(start) => {
                            if now.duration_since(start).as_secs() >= self.shutdown_timeout_secs {
                                // Final check
                                if let Ok(final_count) = get_player_count(&target).await
                                    && final_count <= self.min_players
                                {
                                    if let Some(ref handler) = self.handler {
                                        if let Err(e) = handler.on_shutdown().await {
                                            error!("Auto-shutdown handler failed: {e}");
                                        } else {
                                            tracing::info!("Auto-shutdown triggered successfully");
                                            if let Some(ref url) = self.shutdown_webhook {
                                                send_webhook(url, &format!("Server shut down due to inactivity (players: {final_count})"));
                                            }
                                        }
                                    }
                                    empty_since = None;
                                }
                            }
                        }
                        None => empty_since = Some(now),
                    }
                }
                Ok(_) | Err(_) => empty_since = None,
            }
        }
    }

    async fn handle_connection(self: Arc<Self>, mut stream: TcpStream) -> Result<()> {
        if self.debug && let Ok(addr) = stream.peer_addr() {
            tracing::info!("New Java connection from {}", addr);
        }
        let any_online = self.check_any_online().await;

        if any_online {
            self.is_waking.store(false, Ordering::SeqCst);
            return proxy_to_backend(&mut stream, &self.targets, None).await;
        }

        let raw_packet = read_raw_packet(&mut stream).await?;
        let handshake = parse_handshake(&raw_packet)?;

        match handshake.next_state {
            1 => self.handle_status(&mut stream, &handshake).await,
            2 => self.handle_login(&mut stream, &handshake).await,
            _ => proxy_to_backend(&mut stream, &self.targets, Some(&raw_packet)).await,
        }
    }

    async fn check_any_online(&self) -> bool {
        for target in &self.targets {
            if timeout(Duration::from_millis(500), TcpStream::connect(target))
                .await
                .ok()
                .and_then(std::result::Result::ok)
                .is_some()
            {
                return true;
            }
        }
        false
    }

    async fn handle_status(&self, stream: &mut TcpStream, _handshake: &Handshake) -> Result<()> {
        if matches!(self.startup_on, StartupOn::Ping | StartupOn::Always)
            && self.is_waking.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            && let Some(ref handler) = self.handler
        {
            handler.on_startup().await?;
            if let Some(ref url) = self.startup_webhook {
                send_webhook(url, "Server starting up (triggered by Ping)");
            }
        }

        let _request = read_raw_packet(stream).await?;
        let motd = self.offline_motd.as_deref().unwrap_or(DEFAULT_MOTD);
        write_status_response(stream, motd).await?;

        let ping = read_raw_packet(stream).await?;
        stream.write_all(&ping).await.map_err(EnderError::Io)?;

        Ok(())
    }

    async fn handle_login(&self, stream: &mut TcpStream, _handshake: &Handshake) -> Result<()> {
        if self.is_waking.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
            && let Some(ref handler) = self.handler
        {
            handler.on_startup().await?;
            if let Some(ref url) = self.startup_webhook {
                send_webhook(url, "Server starting up (triggered by Join)");
            }
        }

        let msg = self.offline_message.as_deref().unwrap_or(DEFAULT_DISCONNECT);
        let mut packet = encode_varint(0x00);
        packet.extend_from_slice(&encode_mc_packet(msg.as_bytes())?);
        stream.write_all(&encode_mc_packet(&packet)?).await.map_err(EnderError::Io)?;
        
        Ok(())
    }
}

async fn write_status_response(stream: &mut TcpStream, json_motd: &str) -> Result<()> {
    let mut packet = encode_varint(0x00);
    packet.extend_from_slice(&encode_mc_packet(json_motd.as_bytes())?);
    stream.write_all(&encode_mc_packet(&packet)?).await.map_err(EnderError::Io)?;
    Ok(())
}

fn parse_handshake(raw: &[u8]) -> Result<Handshake> {
    let mut offset = 0;
    let _length = decode_varint(raw, &mut offset)?;
    let packet_id = decode_varint(raw, &mut offset)?;
    if packet_id != 0x00 {
        return Err(EnderError::PacketParse(format!("Expected handshake packet ID 0x00, got {packet_id:02x}")));
    }

    let proto_ver = decode_varint(raw, &mut offset)?;
    let addr = decode_string(raw, &mut offset)?;
    let mut port_bytes = [0u8; 2];
    port_bytes.copy_from_slice(&raw[offset..offset + 2]);
    let port = u16::from_be_bytes(port_bytes);
    offset += 2;
    let next_state = decode_varint(raw, &mut offset)?;

    Ok(Handshake {
        proto_ver,
        addr,
        port,
        next_state,
        raw: raw.to_vec(),
    })
}

async fn read_raw_packet(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let raw_length = read_varint(stream).await?;
    let length = usize::try_from(raw_length).map_err(|_| EnderError::PacketParse("Negative packet length".into()))?;
    let mut data = vec![0u8; length];
    stream.read_exact(&mut data).await.map_err(EnderError::Io)?;

    encode_mc_packet(&data)
}

async fn proxy_to_backend(client: &mut TcpStream, targets: &[String], initial_packet: Option<&[u8]>) -> Result<()> {
    for target in targets {
        if let Ok(mut backend) = TcpStream::connect(target).await {
            if let Some(pkt) = initial_packet {
                backend.write_all(pkt).await.map_err(EnderError::Io)?;
            }
            let (mut client_read, mut client_write) = client.split();
            let (mut backend_read, mut backend_write) = backend.split();

            let _ = tokio::join!(
                tokio::io::copy(&mut client_read, &mut backend_write),
                tokio::io::copy(&mut backend_read, &mut client_write)
            );
            return Ok(());
        }
    }
    Err(EnderError::Proxy("All targets unreachable".into()))
}

/// Fetches the online player count from a Minecraft server's status endpoint.
///
/// # Errors
///
/// Returns an error if the server is unreachable, returns invalid data, or the ping times out.
pub async fn get_player_count(target: &str) -> Result<usize> {
    let mut stream = timeout(Duration::from_secs(2), TcpStream::connect(target))
        .await
        .map_err(|_| EnderError::Proxy("Timeout connecting to backend for player count".into()))?
        .map_err(EnderError::Io)?;

    let hostname = target.split(':').next().unwrap_or("localhost");
    let mut handshake = encode_varint(0x00);
    handshake.extend_from_slice(&encode_varint(-1));
    handshake.extend_from_slice(&encode_mc_packet(hostname.as_bytes())?);
    handshake.extend_from_slice(&25565u16.to_be_bytes());
    handshake.extend_from_slice(&encode_varint(1));
    stream.write_all(&encode_mc_packet(&handshake)?).await.map_err(EnderError::Io)?;

    let status_req = encode_mc_packet(&encode_varint(0x00))?;
    stream.write_all(&status_req).await.map_err(EnderError::Io)?;

    let response = read_raw_packet(&mut stream).await?;
    let mut offset = 0;
    let _total_len = decode_varint(&response, &mut offset)?;
    let _packet_id = decode_varint(&response, &mut offset)?;
    let json_str = decode_string(&response, &mut offset)?;

    let v: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| EnderError::PacketParse(format!("Invalid status JSON: {e}")))?;
    let online = usize::try_from(v["players"]["online"].as_u64().unwrap_or(0)).unwrap_or(0);
    Ok(online)
}

/// Executes a shell command.
///
/// # Errors
///
/// Returns an error if the command cannot be spawned.
pub fn execute_command(cmd: &str, _wait: bool) -> Result<()> {
    let _child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn()
        .map_err(EnderError::Io)?;
    Ok(())
}

fn send_webhook(url: &str, content: &str) {
    let client = Client::new();
    let body = json!({ "content": content });
    let url = url.to_string();
    tokio::spawn(async move {
        let _ = client.post(url).json(&body).send().await;
    });
}
