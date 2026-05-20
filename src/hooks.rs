use refractium::define_hook;

define_hook!(DebugHook, |ctx, dir, pkt| {
    let session_id = format!("{:016x}", ctx.session_id);
    let direction = match dir {
        refractium::protocols::hooks::Direction::Inbound => ">>",
        refractium::protocols::hooks::Direction::Outbound => "<<",
    };

    // Usamos trace para no spamear el log a menos que se pida explícitamente
    tracing::trace!(
        target: "enderpearl::packets",
        "[{}] {} {} {} bytes",
        session_id,
        direction,
        ctx.protocol,
        pkt.len()
    );
});
