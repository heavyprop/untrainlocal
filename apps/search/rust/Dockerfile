# syntax=docker/dockerfile:1
FROM rust:1.99.0-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends g++ pkg-config libicu-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ src/
COPY cpp/ cpp/

ARG CARGO_BUILD_JOBS=2
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/rust /usr/local/bin/search

FROM builder AS test
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo test --locked --jobs 2

FROM debian:bookworm-slim AS runtime

# ICU and the C++ standard library must match the builder's Debian release.
RUN apt-get update \
    && apt-get install -y --no-install-recommends libicu72 libstdc++6 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/search /usr/local/bin/search
ENV SEARCH_LISTEN_ADDR=0.0.0.0:8080
EXPOSE 8080
USER 10001:10001

# Liveness endpoint: search requests require service authentication.
HEALTHCHECK --interval=10s --timeout=5s --start-period=20s --retries=3 \
    CMD curl --fail --silent --show-error --max-time 3 \
        http://127.0.0.1:8080/health || exit 1

CMD ["/usr/local/bin/search"]
