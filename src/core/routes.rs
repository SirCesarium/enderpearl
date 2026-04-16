use std::collections::HashMap;

pub struct RouteConfig {
    pub tcp: HashMap<String, Vec<String>>,
    pub udp: HashMap<String, Vec<String>>,
}

#[must_use]
pub fn load_routes() -> RouteConfig {
    let tcp = HashMap::new();

    #[cfg(any(feature = "java", feature = "web"))]
    let mut tcp = tcp;

    let udp = HashMap::new();

    #[cfg(feature = "bedrock")]
    let mut udp = udp;

    #[cfg(feature = "java")]
    tcp.insert(
        "minecraftjava".to_string(),
        vec!["127.0.0.1:25566".to_string()],
    );

    #[cfg(feature = "web")]
    tcp.insert("http".to_string(), vec!["127.0.0.1:3000".to_string()]);

    #[cfg(feature = "bedrock")]
    udp.insert(
        "minecraftbedrock".to_string(),
        vec!["127.0.0.1:25566".to_string()],
    );

    RouteConfig { tcp, udp }
}
