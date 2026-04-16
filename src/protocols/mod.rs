#[cfg(feature = "bedrock")]
pub mod bedrock;

#[cfg(feature = "java")]
pub mod java;

#[cfg(feature = "web")]
pub mod web {
    use crate::hooks::packet_logger::PacketLogger;
    use refractium::{Http, hook_protocol};

    hook_protocol!(
        wrapper: HookedHttp,
        proto: Http,
        hooks: [PacketLogger]
    );
}

#[cfg(feature = "bedrock")]
pub use bedrock::MinecraftBedrock;

#[cfg(feature = "java")]
pub use java::MinecraftJava;

#[cfg(feature = "web")]
pub use web::HookedHttp;
