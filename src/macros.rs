#[macro_export]
macro_rules! print_cli {
    ($($arg:tt)*) => {
        #[cfg(feature = "pretty-cli")]
        {
            println!("{}", format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        {
            tracing::info!($($arg)*);
        }
        #[cfg(all(not(feature = "logging"), feature = "pretty-cli"))]
        {
            use owo_colors::OwoColorize;
            println!(" {} {}", " ◆".bright_green(), format!($($arg)*).bright_magenta());
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        {
            tracing::error!($($arg)*);
        }
        #[cfg(all(not(feature = "logging"), feature = "pretty-cli"))]
        {
            use owo_colors::OwoColorize;
            eprintln!(" {} {}", "X".bright_red().bold(), format!($($arg)*).bright_red().bold());
        }
    };
}

#[macro_export]
macro_rules! fail_config {
    ($name:expr, $reason:expr) => {{
        #[cfg(feature = "pretty-cli")]
        {
            use owo_colors::OwoColorize;
            Err($crate::errors::EnderError::Config(
                format!("in upstream {}", $name.bold().bright_red()),
                $reason,
            ))
        }
        #[cfg(not(feature = "pretty-cli"))]
        {
            Err($crate::errors::EnderError::Config(
                format!("in upstream '{}'", $name),
                $reason,
            ))
        }
    }};
}
