use refractium::{Transport, define_protocol};

const RAKNET_MAGIC: &[u8; 16] = b"\x00\xff\xff\x00\xfe\xfe\xfe\xfe\xfd\xfd\xfd\xfd\x12\x34\x56\x78";

#[repr(u8)]
enum BedrockPacket {
    UnconnectedPing = 0x01,
    UnconnectedPong = 0x02,
    OpenConnRequest1 = 0x05,
    OpenConnRequest2 = 0x07,
}

define_protocol!(
    name: MinecraftBedrock,
    transport: Transport::Udp,
    identify: |data| {
        if data.len() < 17 {
            false
        } else {
            match data[0] {
                id if id == BedrockPacket::UnconnectedPing as u8 || id == BedrockPacket::UnconnectedPong as u8 => {
                    data.get(9..25).is_some_and(|m| m == RAKNET_MAGIC)
                },
                id if id == BedrockPacket::OpenConnRequest1 as u8 || id == BedrockPacket::OpenConnRequest2 as u8 => {
                    data.get(1..17).is_some_and(|m| m == RAKNET_MAGIC)
                },
                _ => false,
            }
        }
    }
);
