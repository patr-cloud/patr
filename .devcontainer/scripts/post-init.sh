#!/bin/bash

baseDir=$(dirname $0)

echo "Adding generated certificates to trust chain"
certutil -d sql:$HOME/.pki/nssdb -A -t "P,," -n "patr" -i $baseDir/../volume/nginx-certs/cert.crt
