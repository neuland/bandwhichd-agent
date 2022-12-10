FROM --platform=linux/amd64 rust:slim AS build
RUN set -eux; \
    apt update; \
    apt upgrade --yes
RUN rustup target add x86_64-unknown-linux-gnu
WORKDIR /usr/src/bandwhichd-agent
COPY Cargo.toml Cargo.lock ./
RUN set -eux; \
    mkdir src; \
    echo 'fn main(){}' > src/main.rs; \
    RUSTFLAGS='-C target-feature=+crt-static' cargo build --package bandwhichd-agent --bin bandwhichd-agent --target x86_64-unknown-linux-gnu --release; \
    rm src/main.rs target/x86_64-unknown-linux-gnu/release/deps/bandwhichd_agent*; \
    rmdir src
COPY src ./src
RUN RUSTFLAGS='-C target-feature=+crt-static' cargo build --package bandwhichd-agent --bin bandwhichd-agent --target x86_64-unknown-linux-gnu --release

FROM --platform=linux/amd64 debian:stable-slim AS package
RUN set -eux; \
    apt update; \
    apt upgrade --yes; \
    apt install --yes --no-install-recommends \
    lintian \
    ;
COPY --chown=root:root packaging/debian/files/ ./bandwhichd-agent
COPY --chown=root:root --from=build /usr/src/bandwhichd-agent/target/x86_64-unknown-linux-gnu/release/bandwhichd-agent ./bandwhichd-agent/usr/sbin/bandwhichd-agent
RUN dpkg-deb --build ./bandwhichd-agent
RUN lintian \
    --allow-root \
    --info \
    --suppress-tags maintainer-script-calls-systemctl \
    --suppress-tags no-changelog \
    --suppress-tags no-manual-page \
    --suppress-tags shared-library-lacks-prerequisites \
    bandwhichd-agent.deb
