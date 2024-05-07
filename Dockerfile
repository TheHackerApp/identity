FROM rust:1-bookworm as base
WORKDIR /app

RUN set -eux; \
    apt-get update; \
    apt-get install --yes --no-install-recommends \
      clang \
      libclang-dev \
      libpq-dev \
      libssl-dev \
      pkg-config

RUN mkdir -p ~/.ssh/ && ssh-keyscan ssh.shipyard.rs >> ~/.ssh/known_hosts

ARG MOLD_VERSION=2.31.0
RUN set -eux; \
    wget -qO /tmp/mold.tar.gz https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/mold-${MOLD_VERSION}-x86_64-linux.tar.gz; \
    tar -xf /tmp/mold.tar.gz -C /usr/local --strip-components 1; \
    rm /tmp/mold.tar.gz

FROM base as builder

COPY . .
RUN --mount=type=cache,target=/root/.rustup \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    --mount=type=ssh \
    --mount=type=secret,id=shipyard-token \
    set -eux; \
    export CARGO_REGISTRIES_WAFFLEHACKS_TOKEN=$(cat /run/secrets/shipyard-token); \
    export CARGO_REGISTRIES_WAFFLEHACKS_CREDENTIAL_PROVIDER=cargo:token; \
    export CARGO_NET_GIT_FETCH_WITH_CLI=true; \
    cargo build --release --bin identity; \
    objcopy --compress-debug-sections ./target/release/identity ./identity

FROM debian:bookworm-slim as runtime

RUN set -eux; \
    apt-get update;  \
    apt-get -y install ca-certificates; \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/identity /usr/local/bin

ENV ADDRESS=[::]:4243
ENTRYPOINT ["/usr/local/bin/identity"]
