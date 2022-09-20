#!/bin/bash

set -uex

CLUSTER_NAME=${1:?"Missing parameter: CLUSTER_NAME"}
KUBECONFIG_PATH=${2:?"Missing parameter: KUBECONFIG_PATH"}

if [! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

export KUBECONFIG=$KUBECONFIG_PATH

helm uninstall cert-manager -n=cert-manager
helm uninstall prometheus -n=monitoring
helm uninstall loki -n=monitoring
helm uninstall ingress-nginx -n=ingress-nginx
helm uninstall reflector

kubectl delete ns cert-manager monitoring ingress-nginx
