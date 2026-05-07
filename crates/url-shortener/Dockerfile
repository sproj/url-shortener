FROM rust:1.90 AS builder

RUN rustup target add x86_64-unknown-linux-musl

RUN apt-get update && apt-get install -y musl-tools perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY crates/url-shortener/src ./crates/url-shortener/src
COPY crates/url-shortener/Cargo.toml ./crates/url-shortener/Cargo.toml

COPY crates/auth/Cargo.toml ./crates/auth/Cargo.toml
COPY crates/auth/src ./crates/auth/src

COPY crates/common/Cargo.toml ./crates/common/Cargo.toml
COPY crates/common/src ./crates/common/src

# stub of non-dependency crates (and a main) or cargo gets too sad to build
COPY crates/api-gateway/Cargo.toml ./crates/api-gateway/Cargo.toml
RUN mkdir -p ./crates/api-gateway/src && echo 'fn main() {}' > ./crates/api-gateway/src/main.rs

COPY crates/analytics-consumer/Cargo.toml ./crates/analytics-consumer/Cargo.toml
RUN mkdir -p ./crates/analytics-consumer/src && echo 'fn main() {}' > ./crates/analytics-consumer/src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl -p url-shortener

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/url-shortener /usr/local/bin/url-shortener

EXPOSE 8080

CMD ["/usr/local/bin/url-shortener"]
