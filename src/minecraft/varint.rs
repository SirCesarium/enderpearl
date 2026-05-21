use crate::errors::{EnderError, Result};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Encodes an `i32` as a Minecraft `VarInt`.
///
/// The `VarInt` format uses 7 bits per byte, with the high bit set
/// to indicate that more bytes follow.
#[must_use]
#[allow(clippy::cast_sign_loss)]
pub fn encode_varint(value: i32) -> Vec<u8> {
    let mut v = value as u32;
    let mut buf = Vec::with_capacity(5);
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if v == 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_varint_zero() {
        assert_eq!(encode_varint(0), &[0x00]);
    }

    #[test]
    fn encode_varint_single_byte() {
        assert_eq!(encode_varint(127), &[0x7f]);
    }

    #[test]
    fn encode_varint_two_bytes() {
        assert_eq!(encode_varint(128), &[0x80, 0x01]);
    }

    #[test]
    fn encode_varint_max() {
        // -1 as VarInt = 0xFFFFFFFF -> [0xff, 0xff, 0xff, 0xff, 0x0f]
        assert_eq!(encode_varint(-1), &[0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn decode_varint_single_byte() {
        let data = [0x7f];
        let mut offset = 0;
        assert_eq!(decode_varint(&data, &mut offset).unwrap(), 127);
        assert_eq!(offset, 1);
    }

    #[test]
    fn decode_varint_two_bytes() {
        let data = [0x80, 0x01];
        let mut offset = 0;
        assert_eq!(decode_varint(&data, &mut offset).unwrap(), 128);
        assert_eq!(offset, 2);
    }

    #[test]
    fn decode_varint_negative() {
        let data = [0xff, 0xff, 0xff, 0xff, 0x0f];
        let mut offset = 0;
        assert_eq!(decode_varint(&data, &mut offset).unwrap(), -1);
        assert_eq!(offset, 5);
    }

    #[test]
    fn decode_varint_truncated() {
        let data = [0x80];
        let mut offset = 0;
        assert!(decode_varint(&data, &mut offset).is_err());
    }

    #[test]
    fn encode_mc_packet_roundtrip() {
        let payload = b"hello";
        let packet = encode_mc_packet(payload).unwrap();
        // length prefix (varint 5) + payload
        assert_eq!(packet, &[0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn decode_string_valid() {
        // VarInt(5) + "hello"
        let data = [0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
        let mut offset = 0;
        assert_eq!(decode_string(&data, &mut offset).unwrap(), "hello");
        assert_eq!(offset, 6);
    }

    #[test]
    fn decode_string_truncated() {
        let data = [0x05, 0x68, 0x65];
        let mut offset = 0;
        assert!(decode_string(&data, &mut offset).is_err());
    }
}
