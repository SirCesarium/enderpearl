use serde::Serialize;
use std::io::Cursor;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub mod codec;
pub mod handler;

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: Version,
    pub players: Players,
    pub description: Description,
}

#[derive(Serialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize)]
pub struct Players {
    pub max: i32,
    pub online: String,
}

#[derive(Serialize)]
pub struct Description {
    pub text: String,
}

#[derive(Serialize)]
pub struct DisconnectResponse {
    pub text: String,
}

pub struct MinecraftPacket {
    pub id: i32,
    pub data: Vec<u8>,
}

impl MinecraftPacket {
    pub async fn send_json<S: tokio::io::AsyncWriteExt + Unpin, T: serde::Serialize>(
        stream: &mut S,
        id: i32,
        payload: &T,
    ) -> tokio::io::Result<()> {
        let json = serde_json::to_string(payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut data = Vec::new();
        codec::write_string(&mut data, &json);
        let packet = Self { id, data }.serialize();
        stream.write_all(&packet).await?;
        stream.flush().await
    }

    pub fn serialize(self) -> Vec<u8> {
        let mut body = Vec::new();
        codec::write_varint(&mut body, self.id);
        body.extend(self.data);

        let mut frame = Vec::new();
        codec::write_varint(&mut frame, body.len() as i32);
        frame.extend(body);
        frame
    }
}

const MAX_HANDSHAKE_SIZE: usize = 512;

pub async fn inspect_handshake(socket: &TcpStream) -> i32 {
    let mut buf = [0u8; MAX_HANDSHAKE_SIZE];
    
    if let Ok(n) = socket.peek(&mut buf[..]).await {
        let mut cur = Cursor::new(&buf[..n]);

        let _ = codec::read_varint(&mut cur).await;
        if let Ok(0x00) = codec::read_varint(&mut cur).await {
            let _ = codec::read_varint(&mut cur).await;
            let _ = codec::read_string(&mut cur).await;
            let _ = cur.read_u16().await;
            return codec::read_varint(&mut cur).await.unwrap_or(1);
        }
    }
    1
}
