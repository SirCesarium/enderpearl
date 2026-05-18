use refractium::{Transport, define_protocol};

define_protocol!(
    name: MinecraftJava,
    transport: Transport::Tcp,
    identify: |data| {
        if data.len() < 7 {
            false
        } else {
            let packet_len = data[0] as usize;

            match data.get(1) {
                Some(&0x00) if packet_len >= 5 && data.len() > packet_len => {
                    matches!(data.get(packet_len), Some(&1 | &2))
                }
                _ => false,
            }
        }
    }
);
