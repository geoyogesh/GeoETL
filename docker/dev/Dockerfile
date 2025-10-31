# syntax=docker/dockerfile:1.6

ARG RUST_VERSION=1.90
FROM rust:${RUST_VERSION}-bookworm AS base

ARG USER_ID=1000
ARG GROUP_ID=1000
ARG USERNAME=developer

ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:${PATH} \
    CARGO_TERM_COLOR=always

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        curl \
        git \
        libclang-dev \
        libssl-dev \
        libsqlite3-dev \
        llvm \
        pkg-config \
        protobuf-compiler \
        sudo; \
    rm -rf /var/lib/apt/lists/*

# Pre-install helpful cargo utilities
RUN cargo install cargo-watch cargo-nextest
RUN rustup component add rustfmt clippy

# Copy helper scripts for container workflows
COPY docker/dev/scripts/ /opt/geoetl/bin/
RUN chmod +x /opt/geoetl/bin/*.sh
ENV PATH=/opt/geoetl/bin:${PATH}

# Create an unprivileged user that matches the host UID/GID
RUN set -eux; \
    groupadd --gid "${GROUP_ID}" "${USERNAME}"; \
    useradd  --uid "${USER_ID}" --gid "${GROUP_ID}" \
            --shell /bin/bash --create-home "${USERNAME}"; \
    echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/${USERNAME}; \
    chmod 0440 /etc/sudoers.d/${USERNAME}; \
    mkdir -p /workspace /usr/local/cargo /usr/local/rustup; \
    chown -R "${USER_ID}:${GROUP_ID}" /workspace /usr/local/cargo /usr/local/rustup /opt/geoetl

USER ${USERNAME}
WORKDIR /workspace

# Enable incremental builds by default inside the container
ENV CARGO_INCREMENTAL=1

ENTRYPOINT ["/bin/bash"]
