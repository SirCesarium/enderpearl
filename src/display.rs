use std::net::SocketAddr;

#[cfg(feature = "pretty-cli")]
use owo_colors::OwoColorize;

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            println!("{} {}", "!".bright_blue().bold(), format!($($arg)*));
        }
        #[cfg(not(feature = "pretty-cli"))]
        println!("INFO: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            println!("{} {}", "[!]".bright_yellow().bold(), format!($($arg)*).bright_yellow());
        }
        #[cfg(not(feature = "pretty-cli"))]
        println!("WARN: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            eprintln!("{} {}", "X".bright_red().bold(), format!($($arg)*).bright_red().bold());
        }
        #[cfg(not(feature = "pretty-cli"))]
        eprintln!("ERROR: {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            println!("{} {}", ">>".dimmed(), format!($($arg)*).dimmed());
        }
        #[cfg(not(feature = "pretty-cli"))]
        println!("TRACE: {}", format!($($arg)*));
    };
}

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
        println!(
            "{}\n{}",
            BANNER.magenta(),
            "Enderpearl starting up...".magenta().bold().dimmed()
        );
    }

    pub fn print_listen(addr: &SocketAddr) {
        #[cfg(feature = "pretty-cli")]
        println!("{} {}", "Listening on:".bright_green(), addr.underline());

        #[cfg(not(feature = "pretty-cli"))]
        println!("Listening on: {}", addr);
    }

    #[allow(clippy::vec_init_then_push)]
    pub fn print_features() {
        #[allow(unused_mut)]
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
            {
                use owo_colors::OwoColorize;
                print!("{} ", "! Active features:".bold());
                for (i, feat) in features.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}", feat.bright_red());
                }
                println!();
            }

            #[cfg(not(feature = "pretty-cli"))]
            {
                println!("Active features: {}", features.join(", "));
            }
        }
    }
}
