#!/bin/bash

set -uex

KUBECONFIG_PATH=${2:?"Missing parameter: KUBECONFIG_PATH"}

if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

export KUBECONFIG=$KUBECONFIG_PATH

helm uninstall ingress-nginx -n=ingress-nginx
helm uninstall reflector

kubectl delete ns ingress-nginx
