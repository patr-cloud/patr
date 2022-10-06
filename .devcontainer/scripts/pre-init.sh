#!/bin/bash

baseDir=$(dirname $0)

mkdir -p $baseDir/../volume/init-data
mkdir -p $baseDir/../volume/docker-registry/certs
mkdir -p $baseDir/../volume/nginx-certs

# Setup init-data

USER_ID=$(id -u)
GROUP_ID=$(id -g)
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-$USER}

echo "$USER_ID" > $baseDir/../volume/init-data/user
echo "$GROUP_ID" > $baseDir/../volume/init-data/group
echo "COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME" > $baseDir/../../.env

# init docker registry credentials

if [ ! -f $baseDir/../volume/docker-registry/certs/ecdsa.key.pem ]; then
	openssl ecparam -name prime256v1 -genkey -noout -out $baseDir/../volume/docker-registry/certs/ecdsa.key.pem
	## Convert private key to pk8 format
	openssl pkcs8 -topk8 -nocrypt -in $baseDir/../volume/docker-registry/certs/ecdsa.key.pem -out $baseDir/../volume/docker-registry/certs/ecdsa.key.pk8
	## This file has private key. Generate public key to certs/ecdsa.pubkey.pem
	openssl ec -in $baseDir/../volume/docker-registry/certs/ecdsa.key.pem -pubout -out $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.pem
	## Convert public key to DER format
	openssl ec -pubin -inform PEM -in $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.pem -outform DER -out $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.der
	## Generate the certificate with the new key
	openssl req -new -key $baseDir/../volume/docker-registry/certs/ecdsa.key.pem -x509 -nodes -days 365 -out $baseDir/../volume/docker-registry/certs/ecdsa.pubkey.crt -subj "/C=IN/CN=Docker registry"
fi

if [ ! -f $baseDir/../volume/nginx-certs/cert.crt ]; then
	# Setup NGINX certs
	openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout $baseDir/../volume/nginx-certs/privkey.pem -out $baseDir/../volume/nginx-certs/cert.crt -subj "/C=IN/CN=*.patr.cloud"
fi
