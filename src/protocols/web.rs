use crate::hooks::packet_logger::PacketLogger;
use refractium::{hook_protocol, Http};

hook_protocol!(
    wrapper: HookedHttp,
    proto: Http,
    hooks: [PacketLogger]
);
