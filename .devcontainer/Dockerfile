FROM ubuntu:noble

RUN apt update
RUN apt install -y libssl-dev pkg-config bash-completion git sudo curl htop nano vim build-essential postgresql-client clang docker-compose sqlite3 cmake
RUN adduser vscode
RUN adduser vscode sudo
RUN adduser vscode docker
RUN echo vscode ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/vscode
RUN chmod 0440 /etc/sudoers.d/vscode
WORKDIR /tmp/mold
RUN git clone --branch stable https://github.com/rui314/mold.git .
RUN cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=c++ -B build
RUN cmake --build build -j$(nproc)
RUN cmake --build build --target install
RUN rm -rf /tmp/mold
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y --target wasm32-unknown-unknown
RUN /home/vscode/.cargo/bin/cargo install --locked cargo-leptos
RUN /home/vscode/.cargo/bin/cargo install --locked cargo-audit
RUN /home/vscode/.cargo/bin/cargo install --locked sqlx-cli
RUN /home/vscode/.cargo/bin/cargo install --locked worker-build
RUN echo "alias patr='cargo patr'" >> /home/vscode/.bashrc
