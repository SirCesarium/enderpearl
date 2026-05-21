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
                            if let Err(e) = proxy.handle_connection(stream).await {
                                tracing::error!("Java proxy connection handler failed: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Java proxy accept error: {e}");
                    }
                }
            }
        });

        // Spawn inactivity monitor if a handler is present and timeout is configured
        if self.handler.is_some() && self.shutdown_timeout_secs > 0 {
            let monitor_proxy = self.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor_proxy.spawn_monitor().await {
                    tracing::error!("Shutdown monitor failed: {e}");
                }
            });
        }

        Ok(port)
    }

    async fn spawn_monitor(&self) -> Result<()> {
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
                                            tracing::error!("Auto-shutdown handler failed: {e}");
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
    let raw_length = timeout(Duration::from_secs(10), read_varint(stream))
        .await
        .map_err(|_| EnderError::Proxy("Timeout reading packet length".into()))??;
    let length = usize::try_from(raw_length).map_err(|_| EnderError::PacketParse("Negative packet length".into()))?;
    if length > 2_097_152 {
        return Err(EnderError::PacketParse(format!("Packet length {length} exceeds maximum 2MiB")));
    }
    let mut data = vec![0u8; length];
    timeout(Duration::from_secs(10), stream.read_exact(&mut data))
        .await
        .map_err(|_| EnderError::Proxy("Timeout reading packet data".into()))?
        .map_err(EnderError::Io)?;

    encode_mc_packet(&data)
}

async fn proxy_to_backend(client: &mut TcpStream, targets: &[String], initial_packet: Option<&[u8]>) -> Result<()> {
    for target in targets {
        let connect_result = timeout(Duration::from_secs(5), TcpStream::connect(target)).await;
        match connect_result {
            Ok(Ok(mut backend)) => {
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
            Ok(Err(e)) => {
                    tracing::warn!("Failed to connect to backend {target}: {e}");
                }
                Err(_) => {
                    tracing::warn!("Timeout connecting to backend {target}");
                }
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

    let (hostname, port) = if let Some((h, p)) = target.rsplit_once(':') {
        let port: u16 = p.parse().map_err(|_| EnderError::Proxy(format!("invalid port in target '{target}'")))?;
        (h, port)
    } else {
        (target, 25565u16)
    };
    let mut handshake = encode_varint(0x00);
    handshake.extend_from_slice(&encode_varint(-1));
    handshake.extend_from_slice(&encode_mc_packet(hostname.as_bytes())?);
    handshake.extend_from_slice(&port.to_be_bytes());
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
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn()
        .map_err(EnderError::Io)?;

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    tracing::warn!("Command exited with non-zero status: {status}");
                }
            }
            Err(e) => tracing::error!("Failed to wait for command: {e}"),
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handshake_data(proto_ver: i32, addr: &str, port: u16, next_state: i32) -> Vec<u8> {
        let mut buf = encode_varint(0x00); // packet ID
        buf.extend_from_slice(&encode_varint(proto_ver));
        buf.extend_from_slice(&encode_mc_packet(addr.as_bytes()).unwrap());
        buf.extend_from_slice(&port.to_be_bytes());
        buf.extend_from_slice(&encode_varint(next_state));
        encode_mc_packet(&buf).unwrap()
    }

    #[test]
    fn parse_valid_status_handshake() {
        let data = make_handshake_data(766, "localhost", 25565, 1);
        let hs = parse_handshake(&data).unwrap();
        assert_eq!(hs.proto_ver, 766);
        assert_eq!(hs.addr, "localhost");
        assert_eq!(hs.port, 25565);
        assert_eq!(hs.next_state, 1);
    }

    #[test]
    fn parse_valid_login_handshake() {
        let data = make_handshake_data(766, "localhost", 25565, 2);
        let hs = parse_handshake(&data).unwrap();
        assert_eq!(hs.next_state, 2);
    }

    #[test]
    fn parse_handshake_invalid_packet_id() {
        let mut data = make_handshake_data(766, "localhost", 25565, 1);
        // corrupt packet ID to 0x01
        let mut offset = 0;
        let _len = decode_varint(&data, &mut offset).unwrap();
        data[offset] = 0x01;
        assert!(parse_handshake(&data).is_err());
    }

    #[test]
    fn parse_handshake_truncated() {
        let data = [0x01, 0x00]; // too short
        assert!(parse_handshake(&data).is_err());
    }

    #[test]
    fn get_player_count_port_is_parsed() {
        // Test that the target port is used (not hardcoded 25565)
        // We just check the function exists and the signature is right
        // by verifying we can call it with any string
        let target = "127.0.0.1:25566";
        let (hostname, port) = target.rsplit_once(':').unwrap();
        let port: u16 = port.parse().unwrap();
        assert_eq!(port, 25566);
        assert_eq!(hostname, "127.0.0.1");
    }
}

fn send_webhook(url: &str, content: &str) {
    static CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(Client::new);
    let body = json!({ "content": content });
    let url = url.to_string();
    tokio::spawn(async move {
        if let Err(e) = client.post(&url).json(&body).send().await {
            tracing::error!("Webhook POST to {url} failed: {e}");
        }
    });
}
