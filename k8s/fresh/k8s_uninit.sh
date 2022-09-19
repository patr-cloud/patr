#!/bin/bash

helm uninstall cert-manager -n=cert-manager
helm uninstall prometheus -n=monitoring
helm uninstall loki -n=monitoring
helm uninstall ingress-nginx -n=ingress-nginx
helm uninstall reflector

kubectl delete ns cert-manager monitoring ingress-nginx
kubectl delete deploy reflector
