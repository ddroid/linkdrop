FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY linkdrop-core/Cargo.toml linkdrop-core/Cargo.toml
COPY linkdrop-server/Cargo.toml linkdrop-server/Cargo.toml
COPY linkdrop-cli/Cargo.toml linkdrop-cli/Cargo.toml
COPY linkdrop-core/src linkdrop-core/src
COPY linkdrop-server/src linkdrop-server/src
COPY linkdrop-cli/src linkdrop-cli/src
RUN cargo build --release --bin linkdrop-server --bin linkdrop

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/linkdrop-server /usr/local/bin/linkdrop-server
COPY --from=builder /app/target/release/linkdrop /usr/local/bin/linkdrop
ENV PORT=8080
ENV LINKDROP_DATA_DIR=/data
EXPOSE 8080
VOLUME /data
CMD ["linkdrop-server"]
