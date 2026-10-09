# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Build stage: Alpine/musl so the result is a fully static binary.
# `build-base` supplies the C toolchain needed by the -sys crates
# (bzip2-sys, lzma-sys, zstd-sys, ring).
# ---------------------------------------------------------------------------
FROM rust:1.98-alpine3.22 AS builder

RUN apk add --no-cache build-base

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked \
 && strip target/release/coven-scout \
 && mkdir -p /out/workspace

# ---------------------------------------------------------------------------
# Runtime stage: distroless static, non-root (uid 65532), no shell.
# The binary speaks MCP over stdio, so run it with `docker run -i`.
# ---------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

LABEL org.opencontainers.image.title="coven-scout" \
      org.opencontainers.image.description="A fast, safe MCP server for filesystem and web operations" \
      org.opencontainers.image.source="https://github.com/OpenCoven/coven-scout" \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder /src/target/release/coven-scout /usr/local/bin/coven-scout
COPY --from=builder --chown=65532:65532 /out/workspace /workspace

# Sandbox: only /workspace (bind-mount your project here) and /tmp are reachable.
ENV COVEN_SCOUT_ALLOWED_PATHS=/workspace:/tmp

WORKDIR /workspace
USER 65532:65532

ENTRYPOINT ["/usr/local/bin/coven-scout"]
