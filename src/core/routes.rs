use crate::core::types::EnderConfig;
use crate::errors::Result;
use crate::protocols::PROTOCOLS;
use crate::{fail_config, print_cli};
use refractium::Transport;
use refractium::types::{ForwardTarget, ProtocolRoute};

pub fn map_to_refractium(config: &EnderConfig) -> Result<(Vec<ProtocolRoute>, Vec<ProtocolRoute>)> {
    let mut tcp_routes = Vec::new();
    let mut udp_routes = Vec::new();

    for route in &config.upstreams {
        let proto_name = route.protocol.name();

        let proto_meta = PROTOCOLS
            .iter()
            .find(|p| p.id == proto_name || p.aliases.contains(&proto_name.as_str()))
            .ok_or(())
            .or_else(|()| fail_config!(proto_name, "protocol metadata not found".into()))?;

        if !proto_meta.is_enabled {
            return fail_config!(
                proto_name,
                format!("requires '{}' feature", proto_meta.feature)
            );
        }

        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            print_cli!(
                "{} -> {}",
                proto_meta.kind.to_string().bright_cyan(),
                route.targets.join(", ").underline()
            );
        }
        #[cfg(not(feature = "pretty-cli"))]
        print_cli!("{} -> {:?}", proto_meta.kind, route.targets);

        let target = if route.targets.len() == 1 {
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
