FROM rust:1.90 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/url-shortener/src ./crates/url-shortener/src
COPY crates/url-shortener/Cargo.toml ./crates/url-shortener/Cargo.toml

RUN cargo build --release

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/url-shortener /usr/local/bin/url-shortener

EXPOSE 8080

CMD ["/usr/local/bin/url-shortener"]
