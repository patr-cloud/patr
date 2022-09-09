#!/usr/bin/env bash

set -e

baseDir="$(dirname $0)"

certsDir="$baseDir/.volume/certs"
if [ ! -d $certsDir ]; then
    mkdir -p $certsDir

    # Generate private key
    openssl ecparam -name prime256v1 -genkey -noout -out $certsDir/ecdsa.key.pem

    # Convert private key to pk8 format
    openssl pkcs8 -topk8 -nocrypt -in $certsDir/ecdsa.key.pem -out $certsDir/ecdsa.key.pk8

    # This file has private key. Generate public key to certs/ecdsa.pubkey.pem
    openssl ec -in $certsDir/ecdsa.key.pem -pubout -out $certsDir/ecdsa.pubkey.pem

    # Convert public key to DER format
    openssl ec -pubin -inform PEM -in $certsDir/ecdsa.pubkey.pem -outform DER -out $certsDir/ecdsa.pubkey.der

    # Generate the certificate with the new key
    openssl req -new -key $certsDir/ecdsa.key.pem -x509 -nodes -days 365 -out $certsDir/ecdsa.pubkey.crt -subj "/C=IN/CN=Docker registry"
fi
