#!/bin/bash

set -uex

CLUSTER_ID=${1:?"Missing parameter: CLUSTER_ID"}
PARENT_WORKSPACE_ID=${2:?"Missing parameter: PARENT_WORKSPACE_ID"}
KUBECONFIG_PATH=${3:?"Missing parameter: KUBECONFIG_PATH"}

if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

chmod go-r $KUBECONFIG_PATH

export KUBECONFIG=$KUBECONFIG_PATH

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"

echo "Initializing $CLUSTER_ID cluster"

echo "Installing emberstack relfector"
helm upgrade --install reflector emberstack/reflector

echo "Installing nginx as ingress for cluster"
helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx --namespace ingress-nginx --create-namespace

echo "Waiting for nginx ingress controller to be ready"
kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "Ingress controller is ready"

echo "Creating parent workspace in new cluster"
kubectl create namespace "$PARENT_WORKSPACE_ID"

echo "Turn off SSL redirects"
kubectl patch configmap ingress-nginx-controller --namespace ingress-nginx --type strategic --patch '{ "data": { "ssl-redirect": "false" }}'

rm $KUBECONFIG_PATH

echo "Successfully initialized cluster $CLUSTER_ID"
