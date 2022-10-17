# Note: You can use any Debian/Ubuntu based image you want. 
FROM ubuntu:22.04

SHELL ["/bin/bash", "-c"]
RUN apt update
# Install all deps
RUN apt install -y build-essential git pkg-config libssl-dev docker.io postgresql-client sudo sed curl inetutils-ping telnet bash-completion nano dumb-init libnss3-tools ca-certificates jq apt-transport-https software-properties-common

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

# Install kubectl, helm and yq
RUN sudo curl -fsSLo /usr/share/keyrings/kubernetes-archive-keyring.gpg https://packages.cloud.google.com/apt/doc/apt-key.gpg
RUN curl https://baltocdn.com/helm/signing.asc | gpg --dearmor | sudo tee /usr/share/keyrings/helm.gpg > /dev/null
RUN sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys CC86BB64
RUN sudo add-apt-repository ppa:rmescandon/yq
RUN echo "deb [signed-by=/usr/share/keyrings/kubernetes-archive-keyring.gpg] https://apt.kubernetes.io/ kubernetes-xenial main" | sudo tee /etc/apt/sources.list.d/kubernetes.list
RUN echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/helm.gpg] https://baltocdn.com/helm/stable/debian/ all main" | sudo tee /etc/apt/sources.list.d/helm-stable-debian.list
RUN sudo apt update
RUN sudo apt install -y kubectl helm yq
RUN echo 'source <(kubectl completion bash)' >> /home/vscode/.bashrc

# Create a bash_history file
RUN touch /home/vscode/.bash_history

CMD [ "sudo", "/bin/dumb-init", "/runc.sh" ]
