use crate::errors::{EnderError, Result};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Encodes an `i32` as a Minecraft `VarInt`.
///
/// The `VarInt` format uses 7 bits per byte, with the high bit set
/// to indicate that more bytes follow.
#[must_use]
pub fn encode_varint(mut value: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    loop {
        #[allow(clippy::cast_sign_loss)]
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

/// Decodes a Minecraft `VarInt` from a byte slice.
///
/// # Errors
///
/// Returns an error if the data is truncated or the `VarInt` exceeds 32 bits.
pub fn decode_varint(data: &[u8], offset: &mut usize) -> Result<i32> {
    let mut result = 0i32;
    let mut position = 0;
    loop {
        if *offset >= data.len() {
            return Err(EnderError::PacketParse(
                "Unexpected end of data while decoding VarInt".into(),
            ));
        }
        let byte = data[*offset];
        *offset += 1;
        result |= i32::from(byte & 0x7F) << position;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        position += 7;
        if position >= 32 {
            return Err(EnderError::PacketParse(
                "VarInt too large (exceeds 32 bits)".into(),
            ));
        }
    }
}

/// Reads a Minecraft `VarInt` from an async reader.
///
/// # Errors
///
/// Returns an error on IO failure or if the `VarInt` exceeds 32 bits.
pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32> {
    let mut result = 0i32;
    let mut position = 0;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).await.map_err(EnderError::Io)?;
        let byte = buf[0];
        result |= i32::from(byte & 0x7F) << position;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        position += 7;
        if position >= 32 {
            return Err(EnderError::PacketParse(
                "VarInt too large (exceeds 32 bits)".into(),
            ));
        }
    }
}

/// Encodes a Minecraft packet from a payload: VarInt(length) + payload.
///
/// # Errors
///
/// Returns an error if the payload length exceeds `i32::MAX`.
pub fn encode_mc_packet(payload: &[u8]) -> Result<Vec<u8>> {
    let len = i32::try_from(payload.len())
        .map_err(|_| EnderError::PacketParse("Minecraft packet payload exceeds i32::MAX".into()))?;
    let mut packet = encode_varint(len);
    packet.extend_from_slice(payload);
    Ok(packet)
}

/// Decodes a length-prefixed UTF-8 string from a byte slice.
///
/// The length is encoded as a `VarInt` followed by that many UTF-8 bytes.
///
/// # Errors
///
/// Returns an error if the `VarInt` is negative, the string exceeds the data,
/// or the bytes are not valid UTF-8.
pub fn decode_string(data: &[u8], offset: &mut usize) -> Result<String> {
    let len = usize::try_from(decode_varint(data, offset)?)
        .map_err(|_| EnderError::PacketParse("Negative VarInt length for string".into()))?;
    if *offset + len > data.len() {
        return Err(EnderError::PacketParse("String length exceeds remaining data".into()));
    }
    let s = String::from_utf8(data[*offset..*offset + len].to_vec())
        .map_err(|e| EnderError::PacketParse(format!("Invalid UTF-8 in string: {e}")))?;
    *offset += len;
    Ok(s)
}
