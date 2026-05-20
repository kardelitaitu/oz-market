# syntax=docker/dockerfile:1
# Multi-stage build for marketplace-server
# The binary is fully statically linked (no libpq needed — sqlx uses rustls).

# ---- Builder stage ----
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/ backend/
COPY docs/ docs/

WORKDIR /app/backend
RUN cargo build --release --package marketplace-server

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# The binary
COPY --from=builder /app/backend/target/release/marketplace-server /app/marketplace-server

# OpenAPI spec (served at runtime from filesystem)
COPY --from=builder /app/docs/specs/openapi.yaml /app/docs/specs/openapi.yaml

EXPOSE 3000

ENV MARKETPLACE_BIND=0.0.0.0:3000

CMD ["/app/marketplace-server"]
