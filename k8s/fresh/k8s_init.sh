#!/bin/bash

set -ue

CLUSTER_NAME=${1:?"Missing parameter: CLUSTER_NAME"}
KUBECONFIG_PATH=${2:?"Missing parameter: KUBECONFIG_PATH"}

if [! -f $KUBECONFIG_PATH ]; then 
    echo "Kubeconfig file not found: $KUBECONFIG_PATH"
    exit 1
fi

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"

echo "***********************************************************"
echo "Initializing $CLUSTER_NAME cluster"
echo "***********************************************************"

echo "***********************************************************"
echo "Installing emberstack relfector using helm"
echo "***********************************************************"

helm upgrade --install reflector emberstack/reflector

echo "***********************************************************"
echo "Successfully installed reflector"
echo "***********************************************************"

echo "***********************************************************"
echo "Installing cert-manager using helm"
echo -e "***********************************************************\n"

helm upgrade --install cert-manager jetstack/cert-manager --namespace cert-manager --create-namespace --set installCRDs=true

echo "***********************************************************"
echo "Successfully installed cert-manager"
echo "***********************************************************"

echo "***********************************************************"
echo "Waiting for cert-manager to be ready"
echo "***********************************************************"

kubectl wait --namespace cert-manager --for=condition=available deployment --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace cert-manager --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "***********************************************************"
echo "cert-manager is ready"
echo "***********************************************************"

echo "***********************************************************"
echo "Setting up certificate and secrets for the cluster"
echo -e "***********************************************************\n"

kubectl apply -f $CONFIG_DIR/cloudflare-api-token.yaml
sleep 2
kubectl apply -f $CONFIG_DIR/cluster-issuer.yaml
sleep 2
kubectl apply -f $CONFIG_DIR/wildcard-cluster-certificate.yaml
sleep 5

echo "***********************************************************"
echo "Successfully configured apitoken, cluster issuers and certificates"
echo -e "***********************************************************\n"

helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx --namespace ingress-nginx --create-namespace --set controller.exrtaArgs.default-ssl-certificate="cert-manager/tls-domain-wildcard-patr-cloud"

echo "***********************************************************"
echo "Successfully installed nginx ingress controller"
echo "***********************************************************"

echo "***********************************************************"
echo "Waiting for nginx ingress controller to be ready"
echo "***********************************************************"

kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null
kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=-1s > /dev/null

echo "***********************************************************"
echo "Ingress controller is ready"
echo "***********************************************************"

echo "***********************************************************"
echo "Setting up prometheus using helm"
echo "***********************************************************"

helm upgrade --install prometheus prometheus-community/kube-prometheus-stack --namespace monitoring --create-namespace

echo "***********************************************************"
echo "Successfully installed prometheus"
echo "***********************************************************"

echo "***********************************************************"
echo "Waiting for prometheus to be ready"
echo "***********************************************************"

kubectl wait --namespace monitoring --for=condition=available deployment --selector=app=kube-prometheus-stack-operator --timeout=-1s > /dev/null
kubectl wait --namespace monitoring --for=condition=ready pod --selector=app=kube-prometheus-stack-operator --timeout=-1s > /dev/null

echo "***********************************************************"
echo "prometheus is ready"
echo "***********************************************************"

echo "***********************************************************"
echo "Setting up Loki using helm"
echo "***********************************************************"

helm upgrade --install loki grafana/loki-distributed --namespace monitoring


echo "***********************************************************"
echo "Successfully installed loki"
echo "***********************************************************"

echo "***********************************************************"
echo "Successfully configured all the helm chart required to configure the $CLUSTER_NAME cluster"
echo "***********************************************************"
