#!/bin/bash

set -uex

CLUSTER_ID=${1:?"Missing parameter: CLUSTER_ID"}
PARENT_WORKSPACE_ID=${2:?"Missing parameter: PARENT_WORKSPACE_ID"}
KUBECONFIG_PATH=${3:?"Missing parameter: KUBECONFIG_PATH"}
TLS_CERT_PATH=${4:?"Missing parameter: TLS_CERT_PATH"}
TLS_KEY_PATH=${5:?"Missing parameter: TLS_KEY_PATH"}

DEFAULT_CERT_NAME="default-cert-$CLUSTER_ID"

if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

chmod go-r $KUBECONFIG_PATH

export KUBECONFIG=$KUBECONFIG_PATH

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"

echo "Initializing $CLUSTER_ID cluster"

kubectl create namespace ingress-nginx \
    --dry-run=client -o yaml | kubectl apply -f -

echo "Storing origin CA certificate as secret"
kubectl create secret tls $DEFAULT_CERT_NAME \
    --cert=$TLS_CERT_PATH \
    --key=$TLS_KEY_PATH \
    --namespace ingress-nginx \
    --dry-run=client -o yaml | kubectl apply -f -

echo "Installing nginx as ingress for cluster"
helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx \
    --namespace ingress-nginx --create-namespace \
    --set controller.extraArgs.default-ssl-certificate="ingress-nginx/$DEFAULT_CERT_NAME"

echo "Waiting for nginx ingress controller to be ready"
kubectl wait --namespace ingress-nginx --for=condition=available deployment \
    --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace ingress-nginx --for=condition=ready pod \
    --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "Ingress controller is ready"

echo "Creating parent workspace in new cluster"
kubectl create namespace "$PARENT_WORKSPACE_ID" \
    --dry-run=client -o yaml | kubectl apply -f -

rm $KUBECONFIG_PATH $TLS_CERT_PATH $TLS_KEY_PATH

echo "Successfully initialized cluster $CLUSTER_ID"

echo "Waiting for load balancer to assign IP address..."
