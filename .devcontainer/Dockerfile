# Note: You can use any Debian/Ubuntu based image you want. 
FROM ubuntu:22.04

SHELL ["/bin/bash", "-c"]
RUN apt update
RUN apt install -y build-essential git pkg-config libssl-dev docker.io postgresql-client sudo sed curl inetutils-ping telnet bash-completion

WORKDIR /etc/container-init-data
COPY ./volume/init-data/user user
COPY ./volume/init-data/group group
RUN addgroup --gid $(cat ./group) vscode
RUN adduser --uid $(cat ./user) --gid $(cat ./group) vscode
RUN adduser vscode sudo
RUN sed -i 's/%sudo	ALL=(ALL:ALL) ALL/%sudo	ALL=(ALL:ALL) NOPASSWD:ALL/' /etc/sudoers

USER vscode
WORKDIR /home/vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile default -y
RUN source "$HOME/.cargo/env" && rustup toolchain install nightly
RUN echo "alias psql=\"PGPASSWORD=$PG_PASSWORD psql -U $PGUSER -h $PGHOST\"" >> $HOME/.bashrc

CMD [ "sleep", "infinity" ]