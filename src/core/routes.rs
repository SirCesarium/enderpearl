use crate::core::types::EnderConfig;
use crate::errors::{EnderError, Result};
use crate::protocols::ProtocolKind;
use crate::protocols::ProtocolMeta;
use crate::{fail_config, print_cli};
use refractium::types::{ForwardTarget, ProtocolRoute};
use refractium::Transport;

/// Translates `EnderConfig` upstreams into refractium TCP/UDP route lists.
///
/// Routes with Java protocols that have `fake_motd` or `wake_command` are
/// redirected to the local `JavaProxy` port instead of their original target.
///
/// # Errors
///
/// Returns an error if a protocol is unknown, disabled, or the Java proxy port
/// is missing when required.
pub fn map_to_refractium(config: &EnderConfig) -> Result<(Vec<ProtocolRoute>, Vec<ProtocolRoute>)> {
    let mut tcp_routes = Vec::new();
    let mut udp_routes = Vec::new();

    for route in &config.upstreams {
        let proto_name = route.protocol.name();

        let proto_meta = ProtocolMeta::lookup(&proto_name).ok_or_else(|| {
            EnderError::Config(
                "protocol metadata".into(),
                format!("'{proto_name}' not found"),
            )
        })?;

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

        let is_java = matches!(proto_meta.kind, ProtocolKind::Java);

        let target = if is_java {
            let port = config.java_proxy_port.ok_or_else(|| {
                EnderError::Config("Java proxy".into(), "proxy port not assigned".into())
            })?;
            ForwardTarget::Single(format!("127.0.0.1:{port}"))
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
