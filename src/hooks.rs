use refractium::define_hook;
#[cfg(feature = "logging")]
use refractium::protocols::hooks::Direction;

define_hook!(DebugHook, |ctx, dir, pkt| {
    #[cfg(feature = "logging")]
    {
        let session_id = format!("{:016x}", ctx.session_id);
        let direction = match dir {
            Direction::Inbound => ">>",
            Direction::Outbound => "<<",
        };
        tracing::trace!(
            target: "enderpearl::packets",
            "[{}] {} {} {} bytes",
            session_id,
            direction,
            ctx.protocol,
            pkt.len()
        );
    }

    #[cfg(not(feature = "logging"))]
    {
        let _ = (ctx, dir, pkt);
    }
});
