#!/bin/bash

CLUSTER_NAME=$1

echo "Setting up configuration for $CLUSTER_NAME"

echo "***********************************************************"
echo "Installing helm..."
echo "***********************************************************"

sudo snap install helm --classic

helm version

echo "***********************************************************"
echo "Helm installed successfully"
echo "***********************************************************"

echo "***********************************************************"
echo "Installing kubectl"
echo "***********************************************************"

sudo snap install kubectl --classic
kubectl version

echo "***********************************************************"
echo "Successfully installed kubectl"
echo -e "***********************************************************\n"

echo "***********************************************************"
echo "Setting up helm repo"
echo -e "***********************************************************\n"

echo -e "cert-managern\n"
helm repo add jetstack https://charts.jetstack.io

echo -e "prometheus\n"
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts

echo -e "nginx ingress\n"
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx

echo -e "Grafana loki\n"
helm repo add grafana https://grafana.github.io/helm-charts

echo -e "Banzaicloud-vault\n"
helm repo add banzaicloud-stable https://kubernetes-charts.banzaicloud.com

echo -e "Reflector\n"
helm repo add emberstack https://emberstack.github.io/helm-charts

echo "***********************************************************"
echo "Successfully setup helm repos"
echo "***********************************************************"

echo "***********************************************************"
echo "Updating all helm repos"
echo -e "***********************************************************\n"

helm repo update

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

helm install cert-manager jetstack/cert-manager --namespace cert-manager --create-namespace --set installCRDs=true

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

kubectl apply -f cloudflare-api-token.yaml
sleep 2
kubectl apply -f cluster-issuer.yaml
sleep 2
kubectl apply -f wildcard-cluster-certificate.yaml
sleep 5

echo "***********************************************************"
echo "Successfully configured apitoken, cluster issuers and certificates"
echo -e "***********************************************************\n"

helm install ingress-nginx ingress-nginx/ingress-nginx --namespace ingress-nginx --create-namespace --set controller.exrtaArgs.default-ssl-certificate: "cert-manager/tls-domain-wildcard-patr-cloud"

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

helm install prometheus prometheus-community/kube-prometheus-stack --namespace monitoring --create-namespace

echo "***********************************************************"
echo "Successfully installed prometheus"
echo "***********************************************************"

echo "***********************************************************"
echo "Waiting for prometheus to be ready"
echo "***********************************************************"

kubectl wait --namespace ingress-nginx --for=condition=available deployment --selector=app=kube-prometheus-stack-operator --timeout=-1s > /dev/null
kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app=kube-prometheus-stack-operator --timeout=-1s > /dev/null

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
echo "Setting up vault using banzaicloud config"
echo "***********************************************************"

helm upgrade --install vault-operator banzaicloud-stable/vault-operator --namespace vault --create-namespace
sleep 3
kubectl apply -f ./vault-crd.yaml
sleep 3
kubectl apply -f ./vault-rbac.yaml
sleep 3

echo "***********************************************************"
echo -e "To access the root token use export VAULT_TOKEN=\$(kubectl get secrets vault-unseal-keys -o jsonpath={.data.vault-root} | base64 --decode)"
echo "***********************************************************"

kubectl create ns vault-infra
sleep 2
kubectl label namespace vault-infra name=vault-infra
sleep 2
helm upgrade --namespace vault-infra --install vault-secrets-webhook banzaicloud-stable/vault-secrets-webhook
sleep 5

echo "***********************************************************"
echo "Successfully installed banzai-cloud vault"
echo "***********************************************************"

echo "removing the files and director created"

rm -rf helm* linux-amd64*

echo "***********************************************************"
echo "Successfully configured all the helm chart required to configure the $CLUSTER_NAME cluster"
echo "***********************************************************"
