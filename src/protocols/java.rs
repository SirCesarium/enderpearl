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
                    let state_index = packet_len;

                    match data.get(state_index) {
                        Some(&1 | &2) => {
                            let protocol_ver = data[2];
                            protocol_ver > 0 && protocol_ver < 0x80
                        }
                        _ => false,
                    }
                }
                _ => false,
            }
        }
    }
);
