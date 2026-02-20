FROM ubuntu:noble

# Install dependencies
RUN mkdir -p --mode=0755 /usr/share/keyrings && \
    apt update && \
    apt install -y debian-keyring debian-archive-keyring apt-transport-https curl && \
    \
    # Install Cloudflare
    curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg > /usr/share/keyrings/cloudflare-main.gpg && \
    echo 'deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared noble main' > /etc/apt/sources.list.d/cloudflared.list && \
    \
    # Install Caddy
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg && \
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list && \
    chmod o+r /usr/share/keyrings/caddy-stable-archive-keyring.gpg && \
    chmod o+r /etc/apt/sources.list.d/caddy-stable.list && \
    \
    # Install the rest of the dependencies
    apt update && \
    apt install -y \
    libssl-dev \
    pkg-config \
    bash-completion \
    git \
    sudo \
    curl \
    htop \
    nano \
    vim \
    unzip \
    build-essential \
    postgresql-client \
    clang \
    docker-compose \
    sqlite3 \
    cmake \
    cloudflared \
    musl-tools \
    libpcre3-dev \
    zlib1g-dev \
    mold \
    nginx \
    caddy && \
    redis-tools && \
    rm -rf /var/lib/apt/lists/*

# Setup non-root user with sudo permissions
RUN adduser vscode && \
    adduser vscode sudo && \
    adduser vscode docker && \
    echo vscode ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/vscode && \
    chmod 0440 /etc/sudoers.d/vscode

# Install Rust and tools
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y --target wasm32-unknown-unknown
ENV PATH="/home/vscode/.cargo/bin:${PATH}"
ENV RUSTUP_HOME="/home/vscode/.rustup"
ENV CARGO_HOME="/home/vscode/.cargo"
RUN rustup component add rustc-codegen-cranelift-preview --toolchain nightly

# Install rust-based additional tools
RUN cargo install --locked cargo-binstall
RUN cargo binstall --locked cargo-leptos
RUN cargo binstall --locked cargo-audit
RUN cargo binstall --locked sqlx-cli
RUN cargo binstall --locked wasm-pack
RUN cargo binstall --locked worker-build

RUN curl -fsSL https://fnm.vercel.app/install | bash
ENV PATH="/home/vscode/.local/share/fnm:${PATH}"
RUN fnm install --lts --corepack-enabled

# `patr` should build and run the local CLI
RUN echo "alias patr='cargo patr'" >> /home/vscode/.bashrc
