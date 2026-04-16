use std::net::SocketAddr;

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
        enderpearl::print_cli!("{}\n", BANNER.magenta());
    }

    #[allow(unused)]
    pub fn print_listen(addr: &SocketAddr) {
        #[cfg(feature = "pretty-cli")]
        enderpearl::print_cli!("{} {}\n", "Listening on:".bright_green(), addr.underline());

        #[cfg(not(feature = "pretty-cli"))]
        info!("Listening on: {}\n", addr);
    }

    pub fn print_features() {
        let mut features: Vec<&str> = Vec::new();

        #[cfg(feature = "java")]
        features.push("Java");
        #[cfg(feature = "bedrock")]
        features.push("Bedrock");
        #[cfg(feature = "web")]
        features.push("Web");
        #[cfg(feature = "fake-motd")]
        features.push("Fake-MOTD");
        #[cfg(feature = "wait-list")]
        features.push("Wait-List");

        if !features.is_empty() {
            #[cfg(feature = "pretty-cli")]
            enderpearl::print_cli!(
                "{} {}\n",
                "! Active features:".bold(),
                features.join(", ").bright_red()
            );

            #[cfg(not(feature = "pretty-cli"))]
            info!("Active features: {}\n", features.join(", "));
        }
    }
}
