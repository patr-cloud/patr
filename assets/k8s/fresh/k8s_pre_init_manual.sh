#!/bin/bash

set -uex

# todo: make the kubectl and helm version specific and install it locally to patr

# sudo snap install helm --classic

# helm version

# sudo snap install kubectl --classic
# kubectl version

echo "Adding Cert Manager"
helm repo add jetstack https://charts.jetstack.io

echo -e "Adding Prometheus"
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts

echo -e "Adding Ingress-Nginx"
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx

echo -e "Adding Grafana"
helm repo add grafana https://grafana.github.io/helm-charts

echo -e "Adding Reflector"
helm repo add emberstack https://emberstack.github.io/helm-charts

helm repo update
