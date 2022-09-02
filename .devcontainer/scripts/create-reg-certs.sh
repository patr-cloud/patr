#!/usr/bin/env bash

parent_dir=${1:-$(pwd)}

mkdir -p $parent_dir/certs

# Generate private key
openssl ecparam -name prime256v1 -genkey -noout -out $parent_dir/certs/ecdsa.key.pem

# Convert private key to pk8 format
openssl pkcs8 -topk8 -nocrypt -in $parent_dir/certs/ecdsa.key.pem -out $parent_dir/certs/ecdsa.key.pk8

# This file has private key. Generate public key to certs/ecdsa.pubkey.pem
openssl ec -in $parent_dir/certs/ecdsa.key.pem -pubout -out $parent_dir/certs/ecdsa.pubkey.pem

# Convert public key to DER format
openssl ec -pubin -inform PEM -in $parent_dir/certs/ecdsa.pubkey.pem -outform DER -out $parent_dir/certs/ecdsa.pubkey.der

# Generate the certificate with the new key
openssl req -new -key $parent_dir/certs/ecdsa.key.pem -x509 -nodes -days 365 -out $parent_dir/certs/ecdsa.pubkey.crt -subj "/C=IN/CN=Docker registry"
