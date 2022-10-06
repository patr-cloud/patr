#!/bin/bash

baseDir=$(dirname $0)

echo "Adding generated certificates to trust chain"
cp /workspace/volume/nginx-certs/cert.crt /usr/share/local/ca-certificates/patr-cert.crt
update-ca-certificates

echo "Installing sqlx-cli"
cargo install sqlx-cli

echo "Setting up dev.json"
if [ ! -f /workspace/config/dev.json ]; then
	# Setup config.json
	privateKey=$(cat $baseDir/../volume/docker-registry/certs/ecdsa.key.pem)
	publicKey=$(cat $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.pem)
	publicKeyDer=$(cat $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.der | base64 | tr -d '\n')
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
