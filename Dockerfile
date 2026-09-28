# ─────────────── Build stage ───────────────
FROM rust:1.98-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

# ─────────────── Runtime stage ───────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 botuser
USER botuser

WORKDIR /app
COPY --from=builder /app/target/release/premium-self-bot /app/premium-self-bot

ENV RUST_LOG=info

CMD ["/app/premium-self-bot"]
