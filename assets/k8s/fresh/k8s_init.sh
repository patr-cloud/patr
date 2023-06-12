#!/bin/bash

set -uex

# validate inputs
CLUSTER_ID=${1:?"Missing parameter: CLUSTER_ID"}
PARENT_WORKSPACE_ID=${2:?"Missing parameter: PARENT_WORKSPACE_ID"}
KUBECONFIG_PATH=${3:?"Missing parameter: KUBECONFIG_PATH"}
TLS_CERT_PATH=${4:?"Missing parameter: TLS_CERT_PATH"}
TLS_KEY_PATH=${5:?"Missing parameter: TLS_KEY_PATH"}
AGENT_API_TOKEN=${6:?"Missing parameter: AGENT_API_TOKEN"}

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

echo "Creating parent workspace in new cluster"
kubectl create namespace "$PARENT_WORKSPACE_ID" \
    --dry-run=client -o yaml | kubectl apply -f -

echo "Installing patr agent in new cluster"

kubectl apply -f - <<EOF
apiVersion: v1
kind: Namespace
metadata:
  name: patr-agent-ns
EOF

kubectl apply -f - <<EOF
apiVersion: v1
kind: ServiceAccount
metadata:
  name: patr-agent-sa
  namespace: patr-agent-ns
EOF

# TODO: need to restrict the cluster role scope
kubectl apply -f - <<EOF
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: patr-agent-cr
  namespace: patr-agent-ns
rules:
  - apiGroups:
        - ""
        - apps
        - autoscaling
        - batch
        - extensions
        - policy
        - rbac.authorization.k8s.io
    resources:
      - pods
      - componentstatuses
      - configmaps
      - daemonsets
      - deployments
      - events
      - endpoints
      - horizontalpodautoscalers
      - ingress
      - jobs
      - limitranges
      - namespaces
      - nodes
      - pods
      - persistentvolumes
      - persistentvolumeclaims
      - resourcequotas
      - replicasets
      - replicationcontrollers
      - serviceaccounts
      - services
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
EOF

kubectl apply -f - <<EOF
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: patr-agent-crb
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: patr-agent-cr
subjects:
- kind: ServiceAccount
  name: patr-agent-sa
  namespace: patr-agent-ns
EOF

kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: patr-agent-deploy
  namespace: patr-agent-ns
spec:
  replicas: 1
  selector:
    matchLabels:
      app: patr-agent
  template:
    metadata:
      labels:
        app: patr-agent
    spec:
      serviceAccountName: patr-agent-sa
      containers:
      - name: patr-agent
        image: patrcloud/patr-agent
        env:
          - name: PATR_REGION_ID
            value: $CLUSTER_ID
          - name: PATR_API_TOKEN
            value: $AGENT_API_TOKEN
EOF

rm $KUBECONFIG_PATH $TLS_CERT_PATH $TLS_KEY_PATH

echo "Successfully initialized cluster $CLUSTER_ID"

echo "Waiting for load balancer to assign a host ..."
