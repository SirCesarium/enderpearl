use crate::error;
use crate::core::types::StartupOn;
use crate::errors::{EnderError, Result};
use crate::minecraft::varint::{decode_string, decode_varint, encode_mc_packet, encode_varint, read_varint};
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::io::{copy, AsyncReadExt, AsyncWriteExt};
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
    pub startup_cmd: Option<String>,
    pub startup_on: StartupOn,
    pub offline_motd: Option<String>,
    pub offline_message: Option<String>,
    pub startup_webhook: Option<String>,
    pub shutdown_webhook: Option<String>,
    pub debug: bool,
}

impl JavaProxy {
    /// Binds a TCP listener on `127.0.0.1:0` and spawns the accept loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot be bound.
    pub async fn serve(self: Arc<Self>) -> Result<u16> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let proxy = self.clone();
                        tokio::spawn(async move {
                            let () = proxy.handle_connection(stream).await;
                        });
                    }
                    Err(e) => {
                        error!("Java proxy accept error: {e}");
                    }
                }
            }
        });

        Ok(port)
    }

    async fn handle_connection(self: Arc<Self>, mut stream: TcpStream) {
        let any_online = self.check_any_online().await;

        if any_online {
            if let Err(e) = proxy_to_backend(&mut stream, &self.targets, None).await {
                error!("Java proxy backend: {e:#}");
            }
            return;
        }

        let Ok(raw_packet) = read_raw_packet(&mut stream).await else {
            return;
        };

        let Ok(handshake) = parse_handshake(&raw_packet) else {
            return;
        };

        let result = match handshake.next_state {
            1 => self.handle_status(&mut stream, &handshake).await,
            2 => self.handle_login(&mut stream, &handshake).await,
            _ => proxy_to_backend(&mut stream, &self.targets, Some(&raw_packet)).await,
        };

        if let Err(e) = result {
            if self.debug {
                error!("Java proxy: {e:#}");
            }
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

    async fn handle_status(self: &Arc<Self>, stream: &mut TcpStream, _handshake: &Handshake) -> Result<()> {
        if let Some(ref cmd) = self.startup_cmd {
            if matches!(self.startup_on, StartupOn::Ping | StartupOn::Always) {
                execute_command(cmd, false)?;
                if let Some(ref url) = self.startup_webhook {
                    let _ = send_webhook(url, "Server starting up (triggered by Ping)");
                }
            }
        }

        let _request = read_raw_packet(stream).await?;

        let motd = self.offline_motd.as_deref().unwrap_or(DEFAULT_MOTD);
        write_status_response(stream, motd).await?;

        let ping = read_raw_packet(stream).await?;
        stream.write_all(&ping).await?;

        Ok(())
    }

    async fn handle_login(self: &Arc<Self>, stream: &mut TcpStream, _handshake: &Handshake) -> Result<()> {
        if let Some(ref cmd) = self.startup_cmd {
            if matches!(self.startup_on, StartupOn::Join | StartupOn::Always) {
                execute_command(cmd, false)?;
                if let Some(ref url) = self.startup_webhook {
                    let _ = send_webhook(url, "Server starting up (triggered by Join)");
                }
            }
        }

        let _login_start = read_raw_packet(stream).await?;

        let reason = self.offline_message.as_deref().unwrap_or(DEFAULT_DISCONNECT);
        write_disconnect_response(stream, reason).await
    }
}

async fn read_raw_packet(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let raw_length = read_varint(stream).await?;
    let length = usize::try_from(raw_length)
        .map_err(|_| EnderError::PacketParse("Negative packet length".into()))?;
    let mut data = vec![0u8; length];
    stream.read_exact(&mut data).await.map_err(EnderError::Io)?;

    encode_mc_packet(&data)
}

fn parse_handshake(raw: &[u8]) -> Result<Handshake> {
    let mut offset = 0;
    let _length = decode_varint(raw, &mut offset)?;
    let packet_id = decode_varint(raw, &mut offset)?;
    if packet_id != 0x00 {
        return Err(EnderError::PacketParse(format!(
            "Expected handshake packet (ID 0x00), got 0x{packet_id:02X}"
        )));
    }
    let proto_ver = decode_varint(raw, &mut offset)?;
    let addr = decode_string(raw, &mut offset)?;
    let port = u16::from_be_bytes([raw[offset], raw[offset + 1]]);
    offset += 2;
    let next_state = decode_varint(raw, &mut offset)?;

    Ok(Handshake { proto_ver, addr, port, next_state, raw: raw.to_vec() })
}

async fn write_status_response(stream: &mut TcpStream, motd: &str) -> Result<()> {
    let json_bytes = motd.as_bytes();
    let mut payload = encode_varint(0x00);
    payload.extend_from_slice(&encode_mc_packet(json_bytes)?);

    let packet = encode_mc_packet(&payload)?;
    stream.write_all(&packet).await.map_err(EnderError::Io)?;
    Ok(())
}

async fn write_disconnect_response(stream: &mut TcpStream, reason: &str) -> Result<()> {
    let json_bytes = reason.as_bytes();
    let mut payload = encode_varint(0x00);
    payload.extend_from_slice(&encode_mc_packet(json_bytes)?);

    let packet = encode_mc_packet(&payload)?;
    stream.write_all(&packet).await.map_err(EnderError::Io)?;
    Ok(())
}

pub fn execute_command(cmd: &str, is_shutdown: bool) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if let Some(program) = parts.first() {
        Command::new(program)
            .args(&parts[1..])
            .spawn()
            .map_err(|e| {
                if is_shutdown {
                    EnderError::ShutdownFailure(program.to_string(), e.to_string())
                } else {
                    EnderError::WakeupFailure(program.to_string(), e.to_string())
                }
            })?;
    }
    Ok(())
}

async fn proxy_to_backend(
    stream: &mut TcpStream,
    targets: &[String],
    initial_data: Option<&[u8]>,
) -> Result<()> {
    let target = targets.first()
        .ok_or_else(|| EnderError::NoBackend("No targets configured in Java route".into()))?;
    let mut backend = TcpStream::connect(target)
        .await
        .map_err(|e| EnderError::Proxy(format!("Failed to connect to backend {target}: {e}")))?;
    if let Some(data) = initial_data {
        backend.write_all(data).await.map_err(EnderError::Io)?;
    }
    proxy_bidirectional_raw(stream, &mut backend).await
}

async fn proxy_bidirectional_raw(client: &mut TcpStream, backend: &mut TcpStream) -> Result<()> {
    let (mut cr, mut cw) = client.split();
    let (mut br, mut bw) = backend.split();

    let c2b = copy(&mut cr, &mut bw);
    let b2c = copy(&mut br, &mut cw);

    tokio::select! {
        r = c2b => r,
        r = b2c => r,
    }?;

    Ok(())
}
pub async fn get_player_count(target: &str) -> Result<usize> {
    let mut stream = timeout(Duration::from_secs(2), TcpStream::connect(target))
        .await
        .map_err(|_| EnderError::Proxy("Timeout connecting to backend for player count".into()))?
        .map_err(EnderError::Io)?;

    // 1. Handshake (state 1 = status)
    let hostname = target.split(':').next().unwrap_or("localhost");
    let mut handshake = encode_varint(0x00); // Packet ID
    handshake.extend_from_slice(&encode_varint(-1)); // Protocol version (-1 for modern-ish)
    handshake.extend_from_slice(&encode_mc_packet(hostname.as_bytes())?);
    handshake.extend_from_slice(&25565u16.to_be_bytes()); // Port
    handshake.extend_from_slice(&encode_varint(1)); // Next state: Status
    
    stream.write_all(&encode_mc_packet(&handshake)?).await.map_err(EnderError::Io)?;

    // 2. Status Request
    let status_req = encode_mc_packet(&encode_varint(0x00))?;
    stream.write_all(&status_req).await.map_err(EnderError::Io)?;

    // 3. Read Response
    let response = read_raw_packet(&mut stream).await?;
    let mut offset = 0;
    let _packet_id = decode_varint(&response, &mut offset)?;
    let json_str = decode_string(&response, &mut offset)?;

    // 4. Parse JSON
    let v: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| EnderError::PacketParse(format!("Invalid status JSON: {e}")))?;
    
    let online = v["players"]["online"].as_u64().unwrap_or(0) as usize;
    Ok(online)
}

pub fn send_webhook(url: &str, message: &str) -> Result<()> {
    let url = url.to_string();
    let message = message.to_string();
    
    tokio::spawn(async move {
        let client = Client::new();
        let payload = json!({
            "content": message,
            "username": "Enderpearl Proxy"
        });

        match client.post(&url).json(&payload).send().await {
            Ok(_) => tracing::debug!("Webhook sent successfully to {url}"),
            Err(e) => tracing::error!("Failed to send webhook to {url}: {e}"),
        }
    });

    Ok(())
}
