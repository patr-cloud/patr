#!/bin/bash

# Script to generate keys for our CA

mkdir -p certs/ca certs/csr certs/signed certs/keys

# Generate CA's private key
openssl genrsa -out certs/ca/ca.key 2048

# Generate CA's certificate, and self sign it with the private key
openssl req -new -x509 -key certs/ca/ca.key -out certs/ca/ca.crt

