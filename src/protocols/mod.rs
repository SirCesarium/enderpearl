pub mod bedrock;
pub mod java;

pub use bedrock::MinecraftBedrock;
pub use java::MinecraftJava;
use refractium::{Http, hook_protocol};

use crate::hooks::packet_logger::PacketLogger;

hook_protocol!(
    wrapper: HookedHttp,
    proto: Http,
    hooks: [PacketLogger]
);
