# docker image build --file debian-8-build.Dockerfile --tag bandwhichd-agent:debian-8 .
# docker container create --name bandwhichd-agent-debian-8 bandwhichd-agent:debian-8
# docker container cp bandwhichd-agent-debian-8:/home/build/target/release/bandwhichd-agent ./bandwhichd-agent.debian-8

FROM debian:8-slim
RUN set -eux; \
    apt update; \
    apt upgrade -y; \
    apt install -y \
    apt-utils \
    bash \
    build-essential \
    ca-certificates \
    libssl-dev \
    pkg-config \
    ;
RUN set -eux; \
    groupadd \
    --gid 1000 \
    build; \
    useradd \
    --home-dir /home/build \
    --gid 1000 \
    --create-home \
    --shell /bin/bash \
    --uid 1000 \
    build
USER build
WORKDIR /home/build
ADD --chown=build:build https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init ./rustup-init
RUN set -eux; \
    chmod 755 ./rustup-init; \
    ./rustup-init -y
COPY --chown=build:build . ./
RUN set -eux; \
    . ./.cargo/env; \
    cargo build --package bandwhichd-agent --bin bandwhichd-agent --release
