apk update
apk add ca-certificates
cp /patr-ca-certs/cert.crt /usr/local/share/ca-certificates/patr-cert.crt
update-ca-certificates
/bin/registry serve /etc/docker/registry/config.yml