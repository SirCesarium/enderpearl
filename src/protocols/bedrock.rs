use refractium::{Transport, define_protocol};

define_protocol!(
    name: MinecraftBedrock,
    transport: Transport::Udp,
    identify: |data| {

        if data.is_empty() {
            false
        } else {
            match data[0] {
                0x01 | 0x02 => {
                    let magic = b"\x00\xff\xff\x00\xfe\xfe\xfe\xfe\xfd\xfd\xfd\xfd\x12\x34\x56\x78";
                    data.windows(magic.len()).any(|w| w == magic)
                },
                0x05 | 0x07 => true,
                _ => false,
            }
        }
    }
);
