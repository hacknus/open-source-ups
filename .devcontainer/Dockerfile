FROM node:24-bookworm-slim AS node

FROM ubuntu:24.04
COPY --from=node /usr/local/ /usr/local/

ARG DEBIAN_FRONTEND=noninteractive
ARG USERNAME=ubuntu

ENV LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    VIRTUAL_ENV=/opt/venv \
    PATH=/opt/venv/bin:${PATH} \
    PYTHONUNBUFFERED=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        bubblewrap \
        ca-certificates \
        curl \
        gcc-arm-none-eabi \
        gdb-multiarch \
        git \
        libnewlib-arm-none-eabi \
        libnewlib-dev \
        openocd \
        openssh-client \
        openssl \
        pkg-config \
        procps \
        python3 \
        python3-dev \
        python3-pip \
        python3-venv \
        sudo \
    && rm -rf /var/lib/apt/lists/* \
    && echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}" \
    && chmod 0440 "/etc/sudoers.d/${USERNAME}" \
    && python3 -m venv --system-site-packages "${VIRTUAL_ENV}" \
    && "${VIRTUAL_ENV}/bin/pip" install --upgrade pip setuptools wheel \
    && chown -R "${USERNAME}:${USERNAME}" "${VIRTUAL_ENV}"

# Keep the repository's existing cargo runner working with Ubuntu's ARM GDB.
RUN ln -s /usr/bin/gdb-multiarch /usr/local/bin/arm-none-eabi-gdb

ARG CODEX_VERSION=latest

RUN npm install --global "@openai/codex@${CODEX_VERSION}" \
    && npm cache clean --force \
    && mkdir -p /home/ubuntu/.codex \
    && chown -R "${USERNAME}:${USERNAME}" /home/ubuntu/.codex

COPY .devcontainer/start-codex-server.sh /usr/local/bin/start-codex-server
RUN chmod 0755 /usr/local/bin/start-codex-server

EXPOSE 4500

USER ${USERNAME}

# Install as the IDE user, in the standard location RustRover discovers. ENV
# also exposes the proxies to non-login shells used by IDE build processes.
ENV CARGO_HOME=/home/${USERNAME}/.cargo \
    RUSTUP_HOME=/home/${USERNAME}/.rustup
ENV PATH=${CARGO_HOME}/bin:${PATH}

# The image and project always use the same pinned toolchain. rust-src provides
# the standard-library sources needed by IDE navigation and completion.
COPY --chown=${USERNAME}:${USERNAME} rust-toolchain.toml /tmp/ups-rust/rust-toolchain.toml
WORKDIR /tmp/ups-rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/install-rustup.sh \
    && sh /tmp/install-rustup.sh -y --no-modify-path --profile minimal --default-toolchain none \
    && rustup show \
    && rustup component add rust-src \
    && rustup default "$(python3 -c 'import tomllib; print(tomllib.load(open("rust-toolchain.toml", "rb"))["toolchain"]["channel"])')" \
    && cargo --version \
    && arm-none-eabi-gcc --version \
    && arm-none-eabi-gdb --version

WORKDIR /workspace

CMD ["sleep", "infinity"]
