use refractium::define_hook;

define_hook!(PacketLogger, |ctx, dir, pkt| {
    println!(
        "[{}] Intercepted {:?} packet from {}: {} bytes",
        ctx.session_id,
        dir,
        ctx.client_addr,
        pkt.len()
    );
});
