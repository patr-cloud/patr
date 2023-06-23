# Note: You can use any Debian/Ubuntu based image you want. 
FROM ubuntu:jammy

SHELL ["/bin/bash", "-c"]

# Install all deps
RUN apt update && apt install -y build-essential git pkg-config libssl-dev docker.io postgresql-client sudo sed curl inetutils-ping telnet bash-completion nano dumb-init libnss3-tools ca-certificates jq apt-transport-https software-properties-common

# Setup user accounts
WORKDIR /etc/container-init-data
COPY ./volume/config/init-data/user user
COPY ./volume/config/init-data/group group
COPY ./.bashrc bashrc
COPY ./scripts/runc.sh /runc.sh
RUN addgroup --gid $(cat ./group) vscode
RUN adduser --uid $(cat ./user) --gid $(cat ./group) vscode

# Add newly created user to sudo and docker
RUN adduser vscode sudo
RUN adduser vscode docker
# Make sudo work without a password
RUN sed -i 's/%sudo	ALL=(ALL:ALL) ALL/%sudo	ALL=(ALL:ALL) NOPASSWD:ALL/' /etc/sudoers

USER vscode
WORKDIR /home/vscode

# Install nvm
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.1/install.sh | bash
RUN source /home/vscode/.nvm/nvm.sh && nvm install 16

# Install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile default -y
RUN source "$HOME/.cargo/env" && rustup toolchain install nightly

# Setup bashrc
RUN cat /etc/container-init-data/bashrc >> /home/vscode/.bashrc

# Install homebrew
RUN NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
RUN (echo; echo 'eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"') >> /home/vscode/.bashrc
RUN eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"

# Install kubectl, helm, yq and k3d
RUN /home/linuxbrew/.linuxbrew/bin/brew install kubectl helm yq k3d
RUN echo 'source <(/home/linuxbrew/.linuxbrew/bin/kubectl completion bash)' >> /home/vscode/.bashrc
RUN echo 'source <(/home/linuxbrew/.linuxbrew/bin/k3d completion bash)' >> /home/vscode/.bashrc

# Create a bash_history file
RUN touch /home/vscode/.bash_history

CMD [ "sudo", "/bin/dumb-init", "/runc.sh" ]
