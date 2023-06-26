#!/bin/bash

set -uex

# validate inputs
CLUSTER_ID=${1:?"Missing parameter: CLUSTER_ID"}
PARENT_WORKSPACE_ID=${2:?"Missing parameter: PARENT_WORKSPACE_ID"}
KUBECONFIG_PATH=${3:?"Missing parameter: KUBECONFIG_PATH"}
TLS_CERT_PATH=${4:?"Missing parameter: TLS_CERT_PATH"}
TLS_KEY_PATH=${5:?"Missing parameter: TLS_KEY_PATH"}
PATR_API_TOKEN=${6:?"Missing parameter: PATR_API_TOKEN"}
LOKI_LOG_PUSH_URL=${7:?"Missing parameter: LOKI_LOG_PUSH_URL"}
MIMIR_METRICS_PUSH_URL=${8:?"Missing parameter: MIMIR_METRICS_PUSH_URL"}

# validate input values
if [ ! -f $KUBECONFIG_PATH ]; then
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

if [ ! -f $TLS_CERT_PATH ]; then
    echo "TLS certificate file not found: $TLS_CERT_PATH"
    exit 1
fi

if [ ! -f $TLS_KEY_PATH ]; then
    echo "TLS private key file not found: $TLS_KEY_PATH"
    exit 1
fi

chmod go-r $KUBECONFIG_PATH

export KUBECONFIG=$KUBECONFIG_PATH

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"
DEFAULT_CERT_NAME="default-cert-$CLUSTER_ID"

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

echo "Installing promtail for logs"
helm upgrade --install promtail grafana/promtail --namespace promtail --create-namespace -f - <<EOF
config:
  clients:
    - url: $LOKI_LOG_PUSH_URL
      basic_auth:
        username: $CLUSTER_ID
        password: $PATR_API_TOKEN
  snippets:
    pipelineStages:
      - match:
          selector: '{namespace!~"[a-f0-9]{32}"}'
          action: drop
EOF

echo "Installing prometheus for metrics"

MIMIR_SECRET_NAME="mimr-token-$CLUSTER_ID"

kubectl create namespace prometheus \
  --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic $MIMIR_SECRET_NAME \
  --namespace prometheus \
  --from-literal=username=$CLUSTER_ID \
  --from-literal=password=$PATR_API_TOKEN \
  --dry-run=client -o yaml | kubectl apply -f -

helm upgrade --install prometheus prometheus-community/kube-prometheus-stack --namespace prometheus --create-namespace -f - <<EOF
prometheus:
  prometheusSpec:
    podMonitorSelectorNilUsesHelmValues: false
    serviceMonitorSelectorNilUsesHelmValues: false
    remoteWriteDashboards: true
    hostNetwork: false
    remoteWrite:
      - url: $MIMIR_METRICS_PUSH_URL
        basicAuth:
          username:
            name: $MIMIR_SECRET_NAME
            key: username
          password:
            name: $MIMIR_SECRET_NAME
            key: password
        writeRelabelConfigs:
          - sourceLabels: [namespace]
            regex: "(.*)([a-f0-9]{32})(.*)"
            action: keep
EOF

echo "Creating vault-ingress for cluster"
kubectl create ns vault-infra
kubectl label namespace vault-infra name=vault-infra
helm upgrade --namespace vault-infra --install vault-secrets-webhook banzaicloud-stable/vault-secrets-webhook

echo "Creating parent workspace in new cluster"
kubectl create namespace "$PARENT_WORKSPACE_ID" \
    --dry-run=client -o yaml | kubectl apply -f -

kubectl create secret generic patr-token \
  -n=$PARENT_WORKSPACE_ID \
  --from-literal=token=$PATR_API_TOKEN \
  --dry-run=client -o yaml | kubectl apply -f -

rm $KUBECONFIG_PATH $TLS_CERT_PATH $TLS_KEY_PATH

echo "Successfully initialized cluster $CLUSTER_ID"

echo "Waiting for load balancer to assign a host ..."
