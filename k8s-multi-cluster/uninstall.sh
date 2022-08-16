#!/bin/bash

helm uninstall cert-manager -n=cert-manager
helm uninstall prometheus -n=monitoring
helm uninstall loki -n=monitoring
helm uninstall ingress-nginx -n=ingress-nginx
helm uninstall reflector
helm uninstall vault-secrets-webhook -n=vault-infra


helm repo remove jetstack prometheus-community ingress-nginx grafana banzaicloud-stable emberstack 

kubectl delete ns cert-manager monitoring vault-infra ingress-nginx vault
kubectl delete deploy reflector

