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

echo "Installing cert-manager"
helm upgrade --install cert-manager jetstack/cert-manager --namespace cert-manager --create-namespace --set installCRDs=true

echo "Waiting for cert-manager to be ready"
kubectl wait --namespace cert-manager --for=condition=available deployment --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace cert-manager --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "Setup cert-manager with ACME HTTP01 challenge"
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod-http
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: postmaster+$CLUSTER_ID@vicara.co
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF

echo "Creating parent workspace in new cluster"
kubectl create namespace "$PARENT_WORKSPACE_ID"

rm $KUBECONFIG_PATH

echo "Successfully initialized cluster $CLUSTER_ID"

echo "Waiting for load balancer to assign IP address..."
