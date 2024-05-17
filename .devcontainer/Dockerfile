# Note: You can use any Debian/Ubuntu based image you want. 
FROM ubuntu:jammy

SHELL ["/bin/bash", "-c"]

# Install all deps
RUN apt update && apt install -y \
	build-essential \
	clang \
	git \
	pkg-config \
	libssl-dev \
	docker.io \
	postgresql-client \
	sudo \
	sed \
	curl \
	inetutils-ping \
	telnet \
	bash-completion \
	nano \
	dumb-init \
	libnss3-tools \
	ca-certificates \
	jq \
	apt-transport-https \
	software-properties-common \
	mold

# Setup user accounts
WORKDIR /etc/container-init-data
COPY ./volume/config/init-data/user user
COPY ./volume/config/init-data/group group
COPY ./.bashrc bashrc
COPY ./scripts/runc.sh /runc.sh
RUN adduser --uid $(cat ./user) --gid $(cat ./group) vscode

# Add newly created user to sudo and docker
RUN adduser vscode sudo
RUN adduser vscode docker
# Make sudo work without a password
RUN sed -i 's/%sudo	ALL=(ALL:ALL) ALL/%sudo	ALL=(ALL:ALL) NOPASSWD:ALL/' /etc/sudoers

USER vscode
WORKDIR /home/vscode

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile default -y
RUN source "$HOME/.cargo/env" && rustup toolchain install nightly

# Setup bashrc
RUN cat /etc/container-init-data/bashrc >> /home/vscode/.bashrc

RUN mkdir -p /home/vscode/.bin
RUN echo "cargo run --manifest-path /workspace/cli/Cargo.toml -- $@" > /home/vscode/.bin/patr
RUN chmod +x /home/vscode/.bin/patr
ENV PATH="/home/vscode/.bin:${PATH}"

# Create a bash_history file
RUN touch /home/vscode/.bash_history

CMD [ "sudo", "/bin/dumb-init", "/runc.sh" ]
