#!/bin/bash

baseDir=$(dirname $0)

echo "Adding generated certificates to trust chain"
sudo cp /workspace/.devcontainer/volume/config/nginx-certs/cert.crt /usr/local/share/ca-certificates/patr-cert.crt
sudo update-ca-certificates

if [ -f /workspace/.devcontainer/volume/config/init-data/.bashrc ]; then
	cat /workspace/.devcontainer/volume/config/init-data/.bashrc >> ~/.bashrc
fi

mv -n ~/.cargo/bin ~/.cargo-volume/bin
mv -n ~/.cargo/env ~/.cargo-volume/env
rm -rf ~/.cargo/
ln -s ~/.cargo-volume ~/.cargo

# Setup kubeconfig
source /workspace/.env
mkdir -p ~/.kube

# TODO setup kubernetes cluster with k3d and use that kubeconfig

echo "Installing sqlx-cli"
cargo install sqlx-cli

if [ ! -f /workspace/config/dev.json ]; then
	echo "Setting up dev.json"
	# Setup config.json
	privateKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.key.pem)
	publicKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem)
	publicKeyDer=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.der | base64 | tr -d '\n')

	read -r -d '' jqQuery <<- EOF
	.bindAddress |= "0.0.0.0" |
	.database.host |= "postgres" |
	.database.port |= 5432 |
	.database.user |= "postgres" |
	.database.password |= "postgres" |
	.redis.host |= "redis" |
	.dockerRegistry.serviceName |= "Patr registry service" |
	.dockerRegistry.issuer |= "api.patr.cloud" |
	.dockerRegistry.registryUrl |= "registry.patr.cloud" |
	.dockerRegistry.privateKey |= "$privateKey" |
	.dockerRegistry.publicKey |= "$publicKey" |
	.dockerRegistry.publicKeyDer |= "$publicKeyDer" |
	.dockerRegistry.authorizationHeader |= "authkey123456" |
	.rabbitmq.host |= "rabbitmq" |
	.rabbitmq.username |= "rabbitmq" |
	.rabbitmq.password |= "rabbitmq" |
	EOF
	cat $baseDir/../../config/dev.sample.json | jq "$jqQuery" > $baseDir/../../config/dev.json
fi

echo "Setting up cargo-prepare"
db=$(cat /workspace/config/dev.json | jq '.database.database' | tr -d '"' | tr -d "'")
echo "cargo sqlx prepare --database-url=\"postgres://$PGUSER:$PG_PASSWORD@$PGHOST:5432/$db\" --merged" > ~/.cargo/bin/cargo-prepare
chmod +x ~/.cargo/bin/cargo-prepare
