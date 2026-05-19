use crate::error;
use crate::errors::{EnderError, Result};
use crate::minecraft::varint::{decode_string, decode_varint, encode_mc_packet, encode_varint, read_varint};
use std::net::Ipv4Addr;
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
            error!("Java proxy: {e:#}");
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
        let _request = read_raw_packet(stream).await?;

        let motd = self.fake_motd.as_deref().unwrap_or(DEFAULT_MOTD);
        write_status_response(stream, motd).await?;

        let ping = read_raw_packet(stream).await?;
        stream.write_all(&ping).await?;

        Ok(())
    }

    async fn handle_login(self: &Arc<Self>, stream: &mut TcpStream, handshake: &Handshake) -> Result<()> {
        if let Some(ref cmd) = self.wake_command {
            execute_wake(cmd)?;
        }

        proxy_to_backend(stream, &self.targets, Some(&handshake.raw)).await
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

fn execute_wake(cmd: &str) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if let Some(program) = parts.first() {
        Command::new(program)
            .args(&parts[1..])
            .spawn()
            .map_err(|e| EnderError::WakeupFailure(program.to_string(), e.to_string()))?;
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
