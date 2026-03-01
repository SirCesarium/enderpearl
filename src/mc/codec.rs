use std::io::{Error, ErrorKind};
use tokio::io::AsyncReadExt;

pub async fn read_varint<R: AsyncReadExt + Unpin>(reader: &mut R) -> tokio::io::Result<i32> {
    let mut value = 0;
    let mut position = 0;
    let mut byte_buf = [0u8; 1];
    loop {
        reader.read_exact(&mut byte_buf[..]).await?;
        let byte = byte_buf[0];
        value |= ((byte & 0x7F) as i32) << position;
        if (byte & 0x80) == 0 {
            break;
        }
        position += 7;
        if position >= 32 {
            return Err(Error::new(ErrorKind::InvalidData, "VarInt too big"));
        }
    }
    Ok(value)
}

pub fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7F) == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
}

pub async fn read_string<R: AsyncReadExt + Unpin>(reader: &mut R) -> tokio::io::Result<String> {
    let len = read_varint(reader).await?;
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf[..]).await?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}
