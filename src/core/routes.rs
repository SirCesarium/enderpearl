use std::collections::HashMap;

use crate::core::types::EnderConfig;
use crate::errors::Result;
use crate::protocols::ProtocolMeta;
use crate::{fail_config, print_cli};
use refractium::types::{ForwardTarget, ProtocolRoute};
use refractium::Transport;

/// Translates `EnderConfig` upstreams into refractium TCP/UDP route lists.
///
/// Routes that have a corresponding entry in `proxy_ports` are redirected to
/// the local proxy port instead of their original targets.
///
/// # Errors
///
/// Returns an error if a known protocol is disabled via feature flags.
#[allow(clippy::implicit_hasher)]
pub fn map_to_refractium(
    config: &EnderConfig,
    proxy_ports: &HashMap<String, u16>,
) -> Result<(Vec<ProtocolRoute>, Vec<ProtocolRoute>)> {
    let mut tcp_routes = Vec::new();
    let mut udp_routes = Vec::new();

    for route in &config.upstreams {
        let proto_name = route.protocol.name();

        let proto_meta = ProtocolMeta::lookup(&proto_name);

        if let Some(meta) = proto_meta {
            if !meta.is_enabled {
                return fail_config!(proto_name, format!("requires '{}' feature", meta.feature));
            }

            print_cli!("{} -> {:?}", meta.kind, route.targets);
        }

        let target = if let Some(proxy_port) = proxy_ports.get(&proto_name) {
            ForwardTarget::Single(format!("127.0.0.1:{proxy_port}"))
        } else if route.targets.len() == 1 {
            ForwardTarget::Single(route.targets[0].clone())
        } else {
            ForwardTarget::Multiple(route.targets.clone())
        };

        let proto_route = ProtocolRoute {
            protocol: route.protocol.clone(),
            sni: None,
            forward_to: target,
        };

        match route.protocol.transport() {
            Transport::Tcp => tcp_routes.push(proto_route),
            Transport::Udp => udp_routes.push(proto_route),
            Transport::Both => {
                tcp_routes.push(proto_route.clone());
                udp_routes.push(proto_route);
            }
        }
    }

    Ok((tcp_routes, udp_routes))
}
