FROM ubuntu:noble

RUN apt update
RUN apt install -y libssl-dev pkg-config bash-completion git sudo curl htop nano vim build-essential postgresql-client clang
RUN adduser vscode
RUN adduser vscode sudo
RUN echo vscode ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/vscode
RUN chmod 0440 /etc/sudoers.d/vscode
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y --target wasm32-unknown-unknown
RUN /home/vscode/.cargo/bin/cargo install --locked cargo-leptos
RUN /home/vscode/.cargo/bin/cargo install --locked cargo-audit
RUN /home/vscode/.cargo/bin/cargo install --locked sqlx-cli
