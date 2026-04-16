use crate::core::types::EnderConfig;
use crate::errors::{EnderError, Result};
use crate::print_cli;
use crate::protocols::PROTOCOLS;
use std::collections::HashMap;

pub type RefractiumRoutes = HashMap<String, Vec<String>>;

pub fn map_to_refractium(config: &EnderConfig) -> Result<(RefractiumRoutes, RefractiumRoutes)> {
    let mut tcp = HashMap::new();
    let mut udp = HashMap::new();

    for (name, route) in &config.upstreams {
        let proto = PROTOCOLS
            .iter()
            .find(|p| p.id == name || p.aliases.contains(&name.as_str()))
            .ok_or_else(|| {
                #[cfg(feature = "pretty-cli")]
                {
                    use owo_colors::OwoColorize;
                    EnderError::Config(format!("Unknown protocol: {}", name.bold().bright_red()))
                }
                #[cfg(not(feature = "pretty-cli"))]
                EnderError::Config(format!("Unknown protocol: {name}"))
            })?;

        if !proto.is_enabled {
            return Err(EnderError::Config(format!(
                "Upstream '{}' requires '{}' feature but it is disabled",
                name, proto.feature
            )));
        }

        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            print_cli!(
                "{} -> {}",
                proto.kind.to_string().bright_cyan(),
                route.targets.join(", ").underline()
            );
        }
        #[cfg(not(feature = "pretty-cli"))]
        print_cli!("{} -> {:?}", proto.kind, route.targets);

        tcp.insert(proto.id.to_string(), route.targets.clone());
        udp.insert(proto.id.to_string(), route.targets.clone());
    }

    Ok((tcp, udp))
}
