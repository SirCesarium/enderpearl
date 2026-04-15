use std::collections::HashMap;

pub struct RouteConfig {
    pub tcp: HashMap<String, Vec<String>>,
    pub udp: HashMap<String, Vec<String>>,
}

pub fn load_routes() -> RouteConfig {
    // ROUTE REGISTRY
    let mut tcp = HashMap::new();
    let mut udp = HashMap::new();

    // TCP FW
    tcp.insert(
        "minecraftjava".to_string(),
        vec!["127.0.0.1:25566".to_string()],
    );
    tcp.insert("http".to_string(), vec!["127.0.0.1:3000".to_string()]);

    // UDP FW
    udp.insert(
        "minecraftbedrock".to_string(),
        vec!["127.0.0.1:25566".to_string()],
    );

    RouteConfig { tcp, udp }
}
