#!/bin/bash

# Script to generate keys and sign it using our CA

# Generate private key
openssl genrsa -out certs/keys/example.org.key 2048

# Extract public key from the private key (optional)
openssl rsa -in certs/keys/example.org.key -pubout -out certs/keys/example.org.pubkey

# Generate a CSR
openssl req -new -key certs/keys/example.org.key -out certs/csr/example.org.csr

# Sign the CSR using our CA, and generate a signed crt file
openssl x509 -req -in certs/csr/example.org.csr -CA certs/ca/ca.crt -CAkey certs/ca/ca.key -CAcreateserial -out certs/signed/example.org.crt

# Also generate a cert bundle which also includes the CA's certificate 
cat certs/signed/example.org.crt certs/ca/ca.crt > certs/signed/example.org.bundle.crt

