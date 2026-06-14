use std::net::SocketAddr;

#[cfg(not(feature = "pretty-cli"))]
use crate::info;

#[cfg(feature = "pretty-cli")]
use owo_colors::OwoColorize;

pub struct EnderDisplay;

#[cfg(feature = "pretty-cli")]
const BANNER: &str = r"
                  __                                __
  ___  ____  ____/ /__  _________  ___  ____ ______/ /
 / _ \/ __ \/ __  / _ \/ ___/ __ \/ _ \/ __ `/ ___/ / 
/  __/ / / / /_/ /  __/ /  / /_/ /  __/ /_/ / /  / /  
\___/_/ /_/\__,_/\___/_/  / .___/\___/\__,_/_/  /_/   
                         /_/                          ";

impl EnderDisplay {
    pub fn print_banner() {
        #[cfg(feature = "pretty-cli")]
        crate::print_cli!("{}\n", BANNER.bright_magenta());
    }

    pub fn print_listen(addr: &SocketAddr) {
        #[cfg(feature = "pretty-cli")]
        crate::print_cli!(
            " {} {} {}",
            " ◆".bright_green(),
            "listening on".bright_magenta(),
            addr.to_string().bright_magenta().underline()
        );

        #[cfg(not(feature = "pretty-cli"))]
        info!("Listening on: {}\n", addr);
    }

    pub fn print_features() {
        let mut features: Vec<&str> = Vec::new();

        #[cfg(feature = "java")]
        features.push("Java");

        #[cfg(feature = "web")]
        features.push("Web");

        if !features.is_empty() {
            #[cfg(feature = "pretty-cli")]
            crate::print_cli!(
                " {} {}",
                " ■".bright_green(),
                features.join(", ").bright_magenta()
            );

            #[cfg(not(feature = "pretty-cli"))]
            info!("Active features: {}\n", features.join(", "));
        }
    }
}
