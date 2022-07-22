FROM --platform=linux/amd64 rust:slim AS build
RUN set -eux; \
    apt update; \
    apt upgrade --yes; \
    apt install --yes --no-install-recommends \
    musl-tools \
    ;
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/bandwhichd-agent
COPY Cargo.toml Cargo.lock ./
RUN set -eux; \
    mkdir src; \
    echo 'fn main(){}' > src/main.rs; \
    cargo build --package bandwhichd-agent --bin bandwhichd-agent --target x86_64-unknown-linux-musl --release; \
    rm src/main.rs target/x86_64-unknown-linux-musl/release/deps/bandwhichd_agent*; \
    rmdir src
COPY src ./src
RUN cargo build --package bandwhichd-agent --bin bandwhichd-agent --target x86_64-unknown-linux-musl --release

FROM --platform=linux/amd64 registry.suse.com/bci/bci-base:15.3 AS package
RUN set -eux; \
    zypper --non-interactive install \
    rpm-build \
    rpmlint \
    ;
WORKDIR /usr/src/packages
COPY --chown=root:root packaging/suse/.rpmlintrc packaging/suse/.rpmmacros /root/
COPY --chown=root:root packaging/suse/files/ .
COPY --chown=root:root --from=build /usr/src/bandwhichd-agent/target/x86_64-unknown-linux-musl/release/bandwhichd-agent BUILD/bandwhichd-agent
RUN set -eux; \
    rpmlint --file=/root/.rpmlintrc SPECS/bandwhichd-agent.spec; \
    rpmbuild -bb SPECS/bandwhichd-agent.spec; \
    rpmlint --file=/root/.rpmlintrc RPMS/x86_64/bandwhichd-agent-0.37.0-1.x86_64.rpm