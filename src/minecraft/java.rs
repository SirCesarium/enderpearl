use crate::error;
use crate::minecraft::varint::{decode_string, decode_varint, encode_varint, read_varint};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{copy, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const DEFAULT_MOTD: &str = r#"{"version":{"name":"1.21","protocol":766},"players":{"max":0,"online":0},"description":{"text":"§cServer offline — starting up..."}}"#;

pub struct Handshake {
    pub proto_ver: i32,
    pub addr: String,
    pub port: u16,
    pub next_state: i32,
    pub raw: Vec<u8>,
}

pub struct JavaProxy {
    pub targets: Vec<String>,
    pub wake_command: Option<String>,
    pub fake_motd: Option<String>,
}

impl JavaProxy {
    pub async fn serve(self: Arc<Self>) -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let proxy = self.clone();
                        tokio::spawn(async move {
                            let _ = proxy.handle_connection(stream).await;
                        });
                    }
                    Err(_e) => {
                        error!("Java proxy accept error: {}", _e);
                    }
                }
            }
        });

        Ok(port)
    }

    async fn handle_connection(self: Arc<Self>, mut stream: TcpStream) {
        let any_online = self.check_any_online().await;

        if any_online {
            if let Err(e) = proxy_bidirectional(&mut stream, &self.targets).await {
                error!("Java proxy backend: {:#}", e);
            }
            return;
        }

        let raw_packet = match read_raw_packet(&mut stream).await {
            Ok(pkt) => pkt,
            Err(_) => return,
        };

        let handshake = match parse_handshake(&raw_packet) {
            Ok(h) => h,
            Err(_) => return,
        };

        let result = match handshake.next_state {
            1 => self.handle_status(&mut stream, &handshake).await,
            2 => self.handle_login(&mut stream, &handshake).await,
            _ => skip_to_backend(&mut stream, &raw_packet, &self.targets).await,
        };

        if let Err(e) = result {
            error!("Java proxy: {:#}", e);
        }
    }

    async fn check_any_online(&self) -> bool {
        for target in &self.targets {
            if timeout(Duration::from_millis(500), TcpStream::connect(target))
                .await
                .ok()
                .and_then(|r| r.ok())
                .is_some()
            {
                return true;
            }
        }
        false
    }

    async fn handle_status(self: &Arc<Self>, stream: &mut TcpStream, _handshake: &Handshake) -> Result<()> {
        let _request = read_raw_packet(stream).await?;

        let motd = self.fake_motd.as_deref().unwrap_or(DEFAULT_MOTD);
        write_status_response(stream, motd).await?;

        let ping = read_raw_packet(stream).await?;
        stream.write_all(&ping).await?;

        Ok(())
    }

    async fn handle_login(self: &Arc<Self>, stream: &mut TcpStream, handshake: &Handshake) -> Result<()> {
        if let Some(ref cmd) = self.wake_command {
            execute_wake(cmd).await?;
        }

        skip_to_backend(stream, &handshake.raw, &self.targets).await
    }
}

async fn read_raw_packet(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let length = read_varint(stream).await? as usize;
    let mut data = vec![0u8; length];
    stream.read_exact(&mut data).await?;

    let mut packet = encode_varint(length as i32);
    packet.extend_from_slice(&data);
    Ok(packet)
}

fn parse_handshake(raw: &[u8]) -> Result<Handshake> {
    let mut offset = 0;
    let _length = decode_varint(raw, &mut offset)?;
    let packet_id = decode_varint(raw, &mut offset)?;
    if packet_id != 0x00 {
        anyhow::bail!("Expected handshake packet (ID 0x00), got 0x{packet_id:02X}");
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
    payload.extend_from_slice(&encode_varint(json_bytes.len() as i32));
    payload.extend_from_slice(json_bytes);

    let mut packet = encode_varint(payload.len() as i32);
    packet.extend_from_slice(&payload);

    stream.write_all(&packet).await?;
    Ok(())
}

async fn execute_wake(cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if let Some(program) = parts.first() {
        Command::new(program)
            .args(&parts[1..])
            .spawn()
            .context("Failed to execute wake command")?;
    }
    Ok(())
}

async fn skip_to_backend(stream: &mut TcpStream, handshake: &[u8], targets: &[String]) -> Result<()> {
    let target = targets.first().ok_or_else(|| anyhow::anyhow!("No targets configured"))?;
    let mut backend = TcpStream::connect(target)
        .await
        .with_context(|| format!("Failed to connect to backend {target}"))?;
    backend.write_all(handshake).await?;
    proxy_bidirectional_raw(stream, &mut backend).await
}

async fn proxy_bidirectional(stream: &mut TcpStream, targets: &[String]) -> Result<()> {
    let target = targets.first().ok_or_else(|| anyhow::anyhow!("No targets configured"))?;
    let mut backend = TcpStream::connect(target)
        .await
        .with_context(|| format!("Failed to connect to backend {target}"))?;
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
