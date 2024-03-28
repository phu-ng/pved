FROM rust:1.76.0-bookworm as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
COPY . .
RUN touch src/main.rs
RUN cargo build --release
RUN strip target/release/pved

FROM debian:bookworm-20240311
WORKDIR /app
RUN apt-get update && apt-get install --no-install-recommends -y openssl ca-certificates && apt-get clean autoclean && rm -rf /var/lib/{apt,dpkg,cache,log}/
COPY --from=builder /app/target/release/pved ./
ENTRYPOINT ["/app/pved"]
