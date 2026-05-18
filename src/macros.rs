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
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        {
            tracing::warn!($($arg)*);
        }
        #[cfg(all(not(feature = "logging"), feature = "pretty-cli"))]
        {
            use owo_colors::OwoColorize;
            println!(" {} {}", " ◆".bright_yellow(), format!($($arg)*).bright_yellow());
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
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        {
            tracing::trace!($($arg)*);
        }
        #[cfg(all(not(feature = "logging"), feature = "pretty-cli"))]
        {
            use owo_colors::OwoColorize;
            println!(" {} {}", " ◆".bright_magenta().dimmed(), format!($($arg)*).bright_magenta().dimmed());
        }
    };
}

#[macro_export]
macro_rules! check_feature {
    ($feature:literal, $name:expr, $internal:expr) => {
        if cfg!(feature = $feature) {
            $internal.to_string()
        } else {
            return Err(EnderError::Config(format!(
                "Upstream '{}' requires '{}' feature",
                $name, $feature
            )));
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
