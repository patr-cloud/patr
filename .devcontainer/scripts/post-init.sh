#!/bin/bash

baseDir=$(dirname $0)

echo "Adding generated certificates to trust chain"
sudo cp /workspace/.devcontainer/volume/config/nginx-certs/cert.crt /usr/local/share/ca-certificates/patr-cert.crt
sudo update-ca-certificates

mv -n ~/.cargo/bin ~/.cargo-volume/bin
mv -n ~/.cargo/env ~/.cargo-volume/env
rm -rf ~/.cargo/
ln -s ~/.cargo-volume ~/.cargo

echo "Installing sqlx-cli"
cargo install sqlx-cli

if [ ! -f /workspace/config/dev.json ]; then
	echo "Setting up dev.json"
	# Setup config.json
	privateKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.key.pem)
	publicKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem)
	publicKeyDer=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.der | base64 | tr -d '\n')
	cat $baseDir/../../config/dev.sample.json | \
		jq '.bindAddress |= "0.0.0.0"' | \
		jq '.database.host |= "postgres"' | \
		jq '.database.port |= 5432' | \
		jq '.database.user |= "postgres"' | \
		jq '.database.password |= "postgres"' | \
		jq '.redis.host |= "redis"' | \
		jq '.dockerRegistry.serviceName |= "Patr registry service"' | \
		jq '.dockerRegistry.issuer |= "api.patr.cloud"' | \
		jq '.dockerRegistry.registryUrl |= "registry.patr.cloud"' | \
		jq ".dockerRegistry.privateKey |= \"$privateKey\"" | \
		jq ".dockerRegistry.publicKey |= \"$publicKey\"" | \
		jq ".dockerRegistry.publicKeyDer |= \"$publicKeyDer\"" | \
		jq '.dockerRegistry.authorizationHeader |= "authkey123456"' | \
		jq '.rabbitmq.host |= "rabbitmq"' | \
		jq '.rabbitmq.username |= "rabbitmq"' | \
		jq '.rabbitmq.password |= "rabbitmq"' > $baseDir/../../config/dev.json
fi

echo "Setting up cargo-prepare"
db=$(cat /workspace/config/dev.json | jq '.database.database' | tr -d '"')
echo "cargo sqlx prepare --database-url=\"postgres://$PGUSER:$PG_PASSWORD@$PGHOST:5432/$db\" --merged" > ~/.cargo/bin/cargo-prepare
chmod +x ~/.cargo/bin/cargo-prepare
