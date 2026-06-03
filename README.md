# enderpearl

[![Crates.io](https://img.shields.io/crates/v/enderpearl?style=flat-square)](https://crates.io/crates/enderpearl)
[![CI](https://img.shields.io/github/actions/workflow/status/SirCesarium/enderpearl/ci.yml?branch=main&style=flat-square)](https://github.com/SirCesarium/enderpearl/actions)
[![License](https://img.shields.io/github/license/SirCesarium/enderpearl?style=flat-square)]()

Async reverse proxy for Minecraft Java and HTTP traffic with automated server lifecycle.

Enderpearl shuts down the backend when idle and wakes it on ping or join.

## Install

```bash
cargo install enderpearl
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/SirCesarium/enderpearl/releases) — `.deb`, `.rpm`, `.msi` included.

Docker:

```bash
docker pull ghcr.io/sircesarium/enderpearl
```

## Quick start

```bash
enderpearl init              # interactive config wizard
enderpearl                   # start proxy (reads enderpearl.toml)
```

## Configuration

```toml
[server]
bind = "0.0.0.0"
port = 25565

[upstream.minecraft_java]
forward_to = "127.0.0.1:25566"
wake_command = "docker start mc-server"

# Optional:
shutdown_cmd = "docker stop mc-server"
startup_on = "ping"                # join | ping | always
offline_motd = "{...}"             # fake MOTD JSON
offline_message = "{...}"          # fake disconnect JSON
startup_webhook = "https://..."    # POST on wake
shutdown_webhook = "https://..."   # POST on shutdown

[upstream.web]
forward_to = "127.0.0.1:8080"
```

When a client connects, enderpearl checks if the backend is reachable. If online, traffic is proxied straight through. If offline, `JavaProxy` intercepts the Minecraft handshake, responds with a fake MOTD (or disconnect), and runs `wake_command` to start the server. An inactivity monitor polls the backend and runs `shutdown_cmd` after the configured idle timeout.

## CLI

| Command | What |
|---------|------|
| `run` (default) | Start the proxy |
| `init` | Interactive config wizard |

| Flag | What |
|------|------|
| `-c`, `--config` | Config path (default: `enderpearl.toml`) |

## Library usage

Embed enderpearl's proxy engine in your own Rust application:

```bash
cargo add enderpearl --no-default-features --features java
```

```rust
let config = EnderConfig {
    bind: "0.0.0.0".into(),
    port: 25565,
    upstreams: vec![EnderRoute::new(
        ProtocolKind::Java.instantiate(false).unwrap(),
        vec!["127.0.0.1:25566".into()],
    )],
    ..Default::default()
};

let router = EnderRouter::new(&config, &HashMap::new())?;
router.serve("0.0.0.0:25565".parse()?).await?;
```

Custom lifecycle handler — implement `LifecycleHandler` for your own startup/shutdown logic:

```rust
impl LifecycleHandler for MyHandler {
    fn on_startup(&self) -> AsyncResultFuture {
        Box::pin(async { /* docker start, systemctl, etc */ })
    }
    fn on_shutdown(&self) -> AsyncResultFuture {
        Box::pin(async { /* docker stop, etc */ })
    }
}
```

Custom protocols — define new protocol handlers with `ServerProxy`:

```rust
struct BedrockProxy { /* ... */ }

impl ServerProxy for BedrockProxy {
    fn serve(self: Arc<Self>) -> Pin<Box<dyn Future<Output = Result<u16>> + Send>> {
        Box::pin(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let port = listener.local_addr()?.port();
            // accept loop with custom MOTD, wake, etc
            Ok(port)
        })
    }
}
```

Then add it to the config:

```rust
let mut route = EnderRoute::new(Arc::new(MyProtocol), vec!["127.0.0.1:12345".into()]);
route.proxy = Some(Arc::new(BedrockProxy { .. }));
```

The route is redirected to the proxy's local port — refractium handles traffic detection and forwarding.

## Feature flags

| Feature | What |
|---------|------|
| `cli` | CLI parsing (`enderpearl init`, `--config`) + TOML parsing |
| `java` | Minecraft Java protocol detection and `JavaProxy` |
| `web` | HTTP protocol detection |
| `pretty-cli` | Colored output, banners, spinners |
| `logging` | Structured tracing to stderr |

```bash
# Headless server, no colors needed:
cargo install enderpearl --no-default-features --features cli,java,web,logging
```

> [!NOTE]
> Maintenance mode. Bug fixes and dep updates only.

## License

MIT
