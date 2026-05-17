FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-trixie
WORKDIR /app
COPY --from=builder /app/target/release/mcg .
ENTRYPOINT ["./mcg"]
