#!/bin/bash

baseDir=$(dirname $0)

echo "Adding generated certificates to trust chain"
sudo cp /workspace/.devcontainer/volume/config/nginx-certs/cert.crt /usr/local/share/ca-certificates/patr-cert.crt
sudo update-ca-certificates

mv -n ~/.cargo/bin ~/.cargo-volume/bin
mv -n ~/.cargo/env ~/.cargo-volume/env
rm -rf ~/.cargo/
ln -s ~/.cargo-volume ~/.cargo

# Setup kubeconfig
source /workspace/.env
mkdir -p ~/.kube
cp /workspace/.devcontainer/volume/config/init-data/kube-config.yml ~/.kube/config

echo "Installing sqlx-cli"
cargo install sqlx-cli

if [ ! -f /workspace/config/dev.json ]; then
	echo "Setting up dev.json"
	# Setup config.json
	privateKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.key.pem)
	publicKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem)
	publicKeyDer=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.der | base64 | tr -d '\n')
	kubernetesCertificateAuthorityData=$(kubectl config view --output json --raw | jq ".clusters[0].cluster[\"certificate-authority-data\"]")
	clusterAuthToken=$(todo generate a token using service accounts here)

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
		jq '.rabbitmq.password |= "rabbitmq"' | \
		jq ".kubernetes.certificateAuthorityData |= $kubernetesCertificateAuthorityData" | \
		jq ".kubernetes.clusterName |= \"kind-$COMPOSE_PROJECT_NAME\"" | \
		jq '.kubernetes.clusterUrl |= "https://k8s.patr.cloud"' | \
		jq ".kubernetes.authName |= \"kind-$COMPOSE_PROJECT_NAME\"" | \
		jq ".kubernetes.authUsername |= \"kind-$COMPOSE_PROJECT_NAME\"" | \
		jq ".kubernetes.authToken |= \"$clusterAuthToken\"" | \
		jq ".kubernetes.contextName |= \"kind-$COMPOSE_PROJECT_NAME\"" > $baseDir/../../config/dev.json
fi

echo "Setting up cargo-prepare"
db=$(cat /workspace/config/dev.json | jq '.database.database' | tr -d '"')
echo "cargo sqlx prepare --database-url=\"postgres://$PGUSER:$PG_PASSWORD@$PGHOST:5432/$db\" --merged" > ~/.cargo/bin/cargo-prepare
chmod +x ~/.cargo/bin/cargo-prepare

echo "Connecting kind to existing network"
sudo docker -H unix:///var/run/docker-host.sock network connect --alias k8s.patr.cloud ${COMPOSE_PROJECT_NAME}_vpc $COMPOSE_PROJECT_NAME-control-plane || echo "Kind already connected"

# This might be over-automating it. These things can be done manually for now.
# Can be uncommented in the future if this should be automated
# echo "Ensuring that nginx is fully setup in the cluster"
# kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
# kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app.kubernetes.io/component=controller --context kind-$COMPOSE_PROJECT_NAME --timeout=-1s > /dev/null
# kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --context kind-$COMPOSE_PROJECT_NAME --timeout=-1s > /dev/null
