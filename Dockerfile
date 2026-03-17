FROM rust:1.90 AS builder

WORKDIR /app

# RUN apt-get update && apt-get install -y musl-tools libssl-dev
# RUN rustup target add x86_64-unknown-linux-musl

COPY . .

# RUN cargo build --target x86_64-unknown-linux-musl --release
RUN cargo build --release

FROM scratch AS release

COPY --from=builder /app/target/release/url-shortener .
# COPY --from=builder /usr/app/target/x86_64-unknown-linux-musl/release/url-shortener /url-shortener

EXPOSE 8080

CMD ["/url-shortener"]

