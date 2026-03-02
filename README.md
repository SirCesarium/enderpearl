# MC-Gate

![CI](https://github.com/SirCesarium/mc-gate/actions/workflows/ci.yml/badge.svg)
![Release](https://github.com/SirCesarium/mc-gate/actions/workflows/release.yml/badge.svg)

MC-Gate is a high-performance, async proxy designed for Minecraft servers. It acts as a smart gateway that multiplexes HTTP and Minecraft traffic while providing automated server wake-up capabilities.

It solves a common problem: keeping a server offline to save resources, but waking it up automatically the moment a player tries to join or pings the server.

## How it works

MC-Gate performs a non-destructive peek on the first few bytes of every new connection to identify the protocol:

- HTTP Traffic: If the stream starts with a standard method (GET, POST, etc.), it's routed to your web target (optional).

- Minecraft Traffic: If a Minecraft handshake is detected, MC-Gate checks if the backend server is online.

## Smart Wake-up & Waitlist

If the minecraft server is down, MC-Gate doesn't just drop the connection:

- Trigger: It executes a custom command (shell script, API call, etc.) to start your server.

- Condition: You can choose to wake the server when someone simply sees it in their MOTD list or only when they attempt to join.

- Waitlist: While the server boots, MC-Gate holds the player in a "waiting room" for 30 seconds (because Minecraft Client limitations) with real-time status messages, preventing the "Connection Refused" error.

## Key Features

- T-Pipe Multiplexing: Share a single port (e.g., 25565) between your web server and your Minecraft server.

- Auto Wake-up: Automated boot-up on demand.

- Waitlist System: Keeps players connected while the world loads.

- Near-Zero Latency: Once the server is up, traffic is piped directly with minimal overhead using Tokio.

- Library Ready: Can be integrated into Rust GUI applications like Tauri.

## How to use

### Standard execution

Check the [Releases](https://github.com/SirCesarium/mc-gate/releases) page for optimized, standalone binaries.

```
./mcg --listen 0.0.0.0:25565 --mc 127.0.0.1:25567 --on-wakeup "./start_server.sh" --wakeup-on join
```

Parameters

- `--listen`: Address to bind the proxy.

- `--mc`: The real Minecraft server address (internal).

- `--web`: (Optional) Route HTTP traffic to this address.

- `--on-wakeup`: Command to execute when the server needs to start.

- `--wakeup-on`: Trigger condition (motd, join, or disabled).

- `--debug`: Enable detailed connection logs.

## As a Library

MC-Gate is decoupled into a core library and a CLI. You can use the engine in your own Rust projects.

```rust
use mc_gate::{Config, WakeupCondition, run};

// Define your custom wakeup logic (e.g., launching a subprocess or a Docker container)
let callback = Arc::new(|| {
    Box::pin(async move {
        println!("Waking up the server...");
    })
});

let cfg = Arc::new(Config {
    listen: "0.0.0.0:25565".into(),
    web: Some("127.0.0.1:80".into()),
    mc: "127.0.0.1:25567".into(),
    wakeup_on: WakeupCondition::Join,
    on_wakeup: Some(callback),
    // ...
});

run(cfg).await?;
```

### Docker (Official Image)

You don't need to build it yourself. Pull it from GitHub Container Registry:

```
docker run -p 25565:25565 ghcr.io/sircesarium/mc-gate:latest \
  --listen 0.0.0.0:25565 --mc 1.2.3.4:25567 --on-wakeup "./start_server.sh" --wakeup-on join
```

## How to compile

- Make sure to have `rust` and `cargo` installed and updated.

- Run `cargo build --release`

### Docker (local build)

- Build the image: `docker build -t mc-gate .`

- Run it: `docker run -p 25565:25565 mc-gate --listen 0.0.0.0:25565 --mc 1.2.3.4:25567 --on-wakeup "./start_server.sh" --wakeup-on join`
