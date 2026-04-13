use refractium::{Transport, define_protocol};

define_protocol!(
    name: MinecraftJava,
    transport: Transport::Tcp,
    identify: |data| {
        data.len() > 1 && data[1] == 0x00 && data.get(2).is_some_and(|&v| v > 0)
    }
);
