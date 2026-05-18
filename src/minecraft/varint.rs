use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt};

pub fn encode_varint(mut value: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

pub fn decode_varint(data: &[u8], offset: &mut usize) -> Result<i32> {
    let mut result = 0i32;
    let mut position = 0;
    loop {
        if *offset >= data.len() {
            anyhow::bail!("Unexpected end of data while decoding VarInt");
        }
        let byte = data[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as i32) << position;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        position += 7;
        if position >= 32 {
            anyhow::bail!("VarInt too large (exceeds 32 bits)");
        }
    }
}

pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32> {
    let mut result = 0i32;
    let mut position = 0;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).await?;
        let byte = buf[0];
        result |= ((byte & 0x7F) as i32) << position;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        position += 7;
        if position >= 32 {
            anyhow::bail!("VarInt too large (exceeds 32 bits)");
        }
    }
}

pub fn decode_string(data: &[u8], offset: &mut usize) -> Result<String> {
    let len = decode_varint(data, offset)? as usize;
    if *offset + len > data.len() {
        anyhow::bail!("String length exceeds remaining data");
    }
    let s = String::from_utf8(data[*offset..*offset + len].to_vec())?;
    *offset += len;
    Ok(s)
}
