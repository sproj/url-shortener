FROM rust:1.90 AS builder

RUN rustup target add x86_64-unknown-linux-musl

RUN apt-get update && apt-get install -y musl-tools perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY crates/common/Cargo.toml ./crates/common/Cargo.toml
COPY crates/common/src ./crates/common/src

COPY crates/analytics-consumer/Cargo.toml ./crates/analytics-consumer/Cargo.toml
COPY crates/analytics-consumer/src ./crates/analytics-consumer/src

# stub of non-dependency crates (and a main) or cargo gets too sad to build
COPY crates/url-shortener/Cargo.toml ./crates/url-shortener/Cargo.toml
RUN mkdir -p ./crates/url-shortener/src && echo 'fn main() {}' > ./crates/url-shortener/src/main.rs

COPY crates/api-gateway/Cargo.toml ./crates/api-gateway/Cargo.toml
RUN mkdir -p ./crates/url-shortener/src && echo 'fn main() {}' > ./crates/url-shortener/src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl -p analytics-consumer

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/analytics-consumer /usr/local/bin/analytics-consumer

EXPOSE 8080

CMD ["/usr/local/bin/analytics-consumer"]
