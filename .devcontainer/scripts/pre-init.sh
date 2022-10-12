#!/bin/bash

baseDir=$(dirname $0)

mkdir -p $baseDir/../volume/config/init-data
mkdir -p $baseDir/../volume/config/docker-registry
mkdir -p $baseDir/../volume/config/nginx-certs
mkdir -p $baseDir/../volume/data/cargo
mkdir -p $baseDir/../volume/data/docker-registry
mkdir -p $baseDir/../volume/data/postgres
mkdir -p $baseDir/../volume/data/dockerd

# Setup init-data

USER_ID=$(id -u)
GROUP_ID=$(id -g)
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-$USER}

echo "$USER_ID" > $baseDir/../volume/config/init-data/user
echo "$GROUP_ID" > $baseDir/../volume/config/init-data/group
echo "COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME" > $baseDir/../../.env

# init docker registry credentials

if [ ! -f $baseDir/../volume/config/docker-registry/ecdsa.key.pem ]; then
	openssl ecparam -name prime256v1 -genkey -noout -out $baseDir/../volume/config/docker-registry/ecdsa.key.pem
	## Convert private key to pk8 format
	openssl pkcs8 -topk8 -nocrypt -in $baseDir/../volume/config/docker-registry/ecdsa.key.pem -out $baseDir/../volume/config/docker-registry/ecdsa.key.pk8
	## This file has private key. Generate public key to certs/ecdsa.pubkey.pem
	openssl ec -in $baseDir/../volume/config/docker-registry/ecdsa.key.pem -pubout -out $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem
	## Convert public key to DER format
	openssl ec -pubin -inform PEM -in $baseDir/../volume/config/docker-registry/ecdsa.pubkey.pem -outform DER -out $baseDir/../volume/config/docker-registry/ecdsa.pubkey.der
	## Generate the certificate with the new key
	openssl req -new -key $baseDir/../volume/config/docker-registry/ecdsa.key.pem -x509 -nodes -days 365 -out $baseDir/../volume/config/docker-registry/ecdsa.pubkey.crt -subj "/C=IN/CN=Docker registry"
fi

if [ ! -f $baseDir/../volume/config/nginx-certs/cert.crt ]; then
	# Setup NGINX certs
	openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout $baseDir/../volume/config/nginx-certs/privkey.pem -out $baseDir/../volume/config/nginx-certs/cert.crt -subj "/C=IN/CN=*.patr.cloud/SAN=*.patr.cloud"
fi