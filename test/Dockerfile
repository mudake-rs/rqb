FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        pkg-config \
        libssl-dev \
        libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
