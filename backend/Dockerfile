# Build context must be the repository root (granit-server depends on the
# granit-api path crate): docker build -f backend/Dockerfile .
FROM rust:1-slim AS builder
WORKDIR /src
# ring / aws-lc-rs need a C toolchain, cmake, and perl to build.
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake perl pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY . .
# Compile-time checked queries use the committed .sqlx data, no database needed.
ENV SQLX_OFFLINE=true
RUN cargo build --release -p granit-server

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/granit-server /usr/local/bin/granit-server
ENTRYPOINT ["granit-server"]
CMD ["serve"]
