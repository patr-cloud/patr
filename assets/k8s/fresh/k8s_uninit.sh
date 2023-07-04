#!/bin/bash

set -uex

# validate inputs
CLUSTER_ID=${1:?"Missing parameter: CLUSTER_ID"}
PARENT_WORKSPACE_ID=${2:?"Missing parameter: PARENT_WORKSPACE_ID"}
KUBECONFIG_PATH=${3:?"Missing parameter: KUBECONFIG_PATH"}

# validate input values
if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

chmod go-r $KUBECONFIG_PATH

export KUBECONFIG=$KUBECONFIG_PATH

echo "Deleting patr workspace from cluster"
kubectl delete namespace "$PARENT_WORKSPACE_ID" \
    --ignore-not-found=true

echo "Deleting promtail for logs if enabled"
helm uninstall promtail -n=promtail || true
kubectl delete namespace promtail \
    --ignore-not-found=true

echo "Deleting promtail for logs if enabled"
helm uninstall prometheus -n=prometheus || true
kubectl delete namespace prometheus \
    --ignore-not-found=true

echo "Deleting ingress from cluster"
helm uninstall ingress-nginx -n=ingress-nginx || true
kubectl delete namespace ingress-nginx \
    --ignore-not-found=true

rm $KUBECONFIG_PATH

echo "Successfully deleted cluster $CLUSTER_ID"
