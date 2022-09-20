#!/bin/bash

set -uex

CLUSTER_NAME=${1:?"Missing parameter: CLUSTER_NAME"}
KUBECONFIG_PATH=${2:?"Missing parameter: KUBECONFIG_PATH"}

if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

export KUBECONFIG=$KUBECONFIG_PATH

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"

echo "Initializing $CLUSTER_NAME cluster"

echo "Installing emberstack relfector using helm"
helm upgrade --install reflector emberstack/reflector

echo "Installing nginx as ingress for cluster"
helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx --namespace ingress-nginx --create-namespace --set controller.exrtaArgs.default-ssl-certificate="cert-manager/tls-domain-wildcard-patr-cloud"

echo "Waiting for nginx ingress controller to be ready"

kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "Ingress controller is ready"

echo "Successfully configured all the helm chart required to configure the $CLUSTER_NAME cluster"
