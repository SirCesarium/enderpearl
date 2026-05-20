use refractium::define_hook;
use crate::print_cli;

#[cfg(feature = "pretty-cli")]
use owo_colors::OwoColorize;

define_hook!(DebugHook, |ctx, dir, pkt| {
    let session_id = format!("{:016x}", ctx.session_id);
    let direction = match dir {
        refractium::protocols::hooks::Direction::Inbound => ">>",
        refractium::protocols::hooks::Direction::Outbound => "<<",
    };

    #[cfg(feature = "pretty-cli")]
    {
        print_cli!(
            "{} {} {} {} bytes",
            session_id.dimmed(),
            direction.bright_yellow(),
            ctx.protocol.bright_cyan(),
            pkt.len().to_string().bright_green()
        );
    }

    #[cfg(not(feature = "pretty-cli"))]
    {
        println!(
            "[{}] {} {} {} bytes",
            session_id,
            direction,
            ctx.protocol,
            pkt.len()
        );
    }
});
