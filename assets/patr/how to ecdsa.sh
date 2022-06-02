mkdir -p certs

# Generate private key
openssl ecparam -name prime256v1 -genkey -noout -out certs/ecdsa.key.pem

# Convert private key to pk8 format
openssl pkcs8 -topk8 -nocrypt -in certs/ecdsa.key.pem -out certs/ecdsa.key.pk8

# This file has private key. Generate public key to certs/ecdsa.pubkey.pem
openssl ec -in certs/ecdsa.key.pem -pubout -out certs/ecdsa.pubkey.pem

# Convert public key to DER format
openssl ec -pubin -inform PEM -in certs/ecdsa.pubkey.pem -outform DER -out certs/ecdsa.pubkey.der

# Generate the certificate with the new key
openssl req -new -key certs/ecdsa.key.pem -x509 -nodes -days 365 -out certs/ecdsa.pubkey.crt -subj "/C=IN/CN=Docker registry"
