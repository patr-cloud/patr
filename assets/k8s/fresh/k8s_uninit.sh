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

echo "Deleting patr workspace from cluster"
kubectl delete namespace "$PARENT_WORKSPACE_ID"

helm uninstall ingress-nginx -n=ingress-nginx
helm uninstall cert-manager -n=cert-manager
helm uninstall reflector

kubectl delete namespace ingress-nginx
kubectl delete ns cert-manager

rm $KUBECONFIG_PATH

echo "Successfully deleted cluster $CLUSTER_ID"

rm $KUBECONFIG_PATH

echo "Successfully deleted cluster $CLUSTER_ID"
