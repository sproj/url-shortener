FROM rust:1.90 AS builder

RUN rustup target add x86_64-unknown-linux-musl

RUN apt-get update && apt-get install -y musl-tools perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY crates/common/Cargo.toml ./crates/common/Cargo.toml
COPY crates/common/src ./crates/common/src

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM debian:bookworm-slim AS release

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/analytics-consumer /usr/local/bin/analytics-consumer

EXPOSE 8080

CMD ["/usr/local/bin/analytics-consumer"]
