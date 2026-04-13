# Enderpearl [WORK IN PROGRESS]

Enderpearl (ex. `mc-gate`) is a lightweight reverse proxy for Minecraft Java & Bedrock written in Rust and powered by [`refractium`](https://github.com/SirCesarium/refractium).

It allows to run Minecraft infrastructure in a **serverless** fashion by automatically managing server lifecycle based on network activity:

  * **Auto-Shutdown**: Powers down the backend servers when no players are connected.
  * **Auto-Wakeup**: Detects incoming connection attempts and triggers server startup before forwarding the traffic.

The project focuses on eliminating resource overhead during idle time by keeping servers offline until they are actually needed, detecting activity at the protocol level to trigger wake-up events.
