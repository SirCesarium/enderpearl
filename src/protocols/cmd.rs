use refractium::{Transport, define_protocol};

define_protocol!(
    name: UdpCommands,
    transport: Transport::Udp,
    identify: |data| {
        let is_cmd = data.starts_with(b"CMD:");
        if is_cmd { println!("Protocol identified: UDP Command"); }
        is_cmd
    }
);
