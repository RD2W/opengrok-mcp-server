# Multi-stage Docker build for opengrok-mcp

# Stage 1: Build
FROM rust:1.97.1-alpine3.24 AS builder
RUN apk add --no-cache musl-dev pkgconf
WORKDIR /build

ARG GIT_HASH
ARG BUILD_DATE
ENV GIT_HASH=${GIT_HASH}
ENV BUILD_DATE=${BUILD_DATE}

COPY Cargo.toml Cargo.lock ./
COPY crates/opengrok-core/Cargo.toml crates/opengrok-core/
COPY crates/opengrok-mcp/Cargo.toml crates/opengrok-mcp/
COPY crates/opengrok-mcp/build.rs crates/opengrok-mcp/
RUN mkdir -p crates/opengrok-core/src crates/opengrok-mcp/src && \
    echo 'fn main() {}' > crates/opengrok-mcp/src/main.rs && \
    echo '' > crates/opengrok-mcp/src/lib.rs && \
    echo '' > crates/opengrok-core/src/lib.rs && \
    cargo build --release && \
    rm -rf target/release/.fingerprint/opengrok-*
COPY crates/opengrok-core/src crates/opengrok-core/src
COPY crates/opengrok-mcp/src crates/opengrok-mcp/src
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:3.24

ARG HEALTHCHECK_PORT=8080
ARG HEALTHCHECK_PATH=/healthz
ARG CONFIG_DIR=/config

RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/opengrok-mcp /usr/local/bin/opengrok-mcp
RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

VOLUME ["${CONFIG_DIR}"]

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD wget -qO- http://localhost:${HEALTHCHECK_PORT}${HEALTHCHECK_PATH} || exit 1

LABEL org.opencontainers.image.title="opengrok-mcp" \
      org.opencontainers.image.description="MCP server for OpenGrok code search (AOSP-scale codebases)" \
      org.opencontainers.image.vendor="RD2W" \
      org.opencontainers.image.authors="Maxim Krutovercev (mkrutovercev@yandex.ru)" \
      org.opencontainers.image.documentation="https://github.com/RD2W/opengrok-mcp-server" \
      org.opencontainers.image.url="https://github.com/RD2W/opengrok-mcp-server" \
      org.opencontainers.image.source="https://github.com/RD2W/opengrok-mcp-server" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"

ENTRYPOINT ["opengrok-mcp"]
