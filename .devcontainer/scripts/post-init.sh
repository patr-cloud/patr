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
cp /workspace/.devcontainer/volume/config/init-data/kube-config.yml ~/.kube/config

echo "Installing sqlx-cli"
cargo install sqlx-cli

echo "Connecting kind to existing network"
sudo docker -H unix:///var/run/docker-host.sock network connect --alias "$COMPOSE_PROJECT_NAME-control-plane" ${COMPOSE_PROJECT_NAME}_vpc $COMPOSE_PROJECT_NAME-control-plane 2> /dev/null || echo "Kind already connected"

if [ ! -f /workspace/config/dev.json ]; then
	echo "Setting up dev.json"
	# Setup config.json
	privateKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.key.pem)
	publicKey=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem)
	publicKeyDer=$(cat $baseDir/../volume/config/docker-registry/ecdsa.pubkey.der | base64 | tr -d '\n')
	kubernetesCertificateAuthorityData=$(kubectl config view --output json --raw | jq ".clusters[0].cluster[\"certificate-authority-data\"]" | tr -d '"')
	kubectl create serviceaccount patr-admin-service-account
	kubectl create clusterrolebinding patr-admin-role-binding --clusterrole=cluster-admin --serviceaccount=default:patr-admin-service-account
	kubectl apply -f - <<EOF
apiVersion: v1
kind: Secret
type: kubernetes.io/service-account-token
metadata:
  name: patr-admin-service-account-token
  annotations:
    kubernetes.io/service-account.name: patr-admin-service-account
EOF
	clusterAuthToken=$(kubectl get secrets patr-admin-service-account-token -o=jsonpath='{.data.token}' | base64 -d)

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
	.kubernetes.certificateAuthorityData |= "$kubernetesCertificateAuthorityData" |
	.kubernetes.clusterName |= "kind-$COMPOSE_PROJECT_NAME" |
	.kubernetes.clusterUrl |= "https://$COMPOSE_PROJECT_NAME-control-plane:6443" |
	.kubernetes.authName |= "patr-admin-service-account" |
	.kubernetes.authUsername |= "patr-admin-service-account" |
	.kubernetes.authToken |= "$clusterAuthToken" |
	.kubernetes.contextName |= "kind-$COMPOSE_PROJECT_NAME"
	EOF
	cat $baseDir/../../config/dev.sample.json | jq "$jqQuery" > $baseDir/../../config/dev.json
fi

echo "Setting up cargo-prepare"
db=$(cat /workspace/config/dev.json | jq '.database.database' | tr -d '"' | tr -d "'")
echo "cargo sqlx prepare --database-url=\"postgres://$PGUSER:$PG_PASSWORD@$PGHOST:5432/$db\" --merged" > ~/.cargo/bin/cargo-prepare
chmod +x ~/.cargo/bin/cargo-prepare

# This might be over-automating it. These things can be done manually for now.
# Can be uncommented in the future if this should be automated
# echo "Ensuring that nginx is fully setup in the cluster"
# kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
# kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app.kubernetes.io/component=controller --context kind-$COMPOSE_PROJECT_NAME --timeout=-1s > /dev/null
# kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --context kind-$COMPOSE_PROJECT_NAME --timeout=-1s > /dev/null
