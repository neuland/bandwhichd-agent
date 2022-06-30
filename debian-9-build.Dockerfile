FROM debian:9-slim AS build
RUN set -eux; \
    apt update; \
    apt upgrade --yes; \
    apt install --yes --no-install-recommends \
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
COPY --chown=build:build Cargo.lock Cargo.toml ./
RUN set -eux; \
    mkdir src; \
    echo 'fn main(){}' > src/main.rs; \
    . ./.cargo/env; \
    cargo build --package bandwhichd-agent --bin bandwhichd-agent --release; \
    rm src/main.rs; \
    rmdir src
COPY --chown=build:build src ./src/
RUN set -eux; \
    . ./.cargo/env; \
    cargo build --package bandwhichd-agent --bin bandwhichd-agent --release

FROM debian:9-slim AS package
RUN set -eux; \
    apt update; \
    apt upgrade --yes; \
    apt install --yes --no-install-recommends \
    lintian \
    ;
COPY --chown=root:root --from=build /home/build/target/release/bandwhichd-agent ./bandwhichd-agent/usr/sbin/bandwhichd-agent
COPY --chown=root:root packaging/debian-9/files/ ./bandwhichd-agent
RUN dpkg-deb --build ./bandwhichd-agent
RUN lintian \
    --allow-root \
    --info \
    --suppress-tags binary-without-manpage \
    --suppress-tags debian-changelog-file-missing \
    --suppress-tags maintainer-script-calls-systemctl \
    bandwhichd-agent.deb