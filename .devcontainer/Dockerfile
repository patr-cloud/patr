FROM ubuntu:noble

# Install dependencies
RUN mkdir -p --mode=0755 /usr/share/keyrings && \
    apt update && \
    apt install -y curl && \
    curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg > /usr/share/keyrings/cloudflare-main.gpg && \
    echo 'deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared noble main' > /etc/apt/sources.list.d/cloudflared.list && \
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
        nginx && \
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

# Install rust-based additional tools
RUN /home/vscode/.cargo/bin/cargo install --locked cargo-binstall
RUN /home/vscode/.cargo/bin/cargo binstall --locked cargo-leptos
RUN /home/vscode/.cargo/bin/cargo binstall --locked cargo-audit
RUN /home/vscode/.cargo/bin/cargo binstall --locked sqlx-cli
RUN /home/vscode/.cargo/bin/cargo binstall --locked wasm-pack
RUN /home/vscode/.cargo/bin/cargo binstall --locked worker-build

RUN curl -fsSL https://fnm.vercel.app/install | bash
RUN ~/.local/share/fnm/fnm install --lts
ENV PATH="/home/vscode/.local/share/fnm/node-versions/v22.20.0/installation/bin:$PATH"
RUN npm install --global corepack@latest
RUN corepack enable pnpm

# `patr` should build and run the local CLI
RUN echo "alias patr='cargo patr'" >> /home/vscode/.bashrc
