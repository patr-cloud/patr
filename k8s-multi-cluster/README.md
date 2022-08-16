### Setting up neccassary configuration to run a cluster

Packages installed
- cert-manager - jetstack/cert-manager
- ingress-nginx - by ingress-nginx/ingress-nginx
- prometheus - by prometheus-community/kube-prometheus-stack
- loki - grafana/loki-distributed
- vault - by banzaicloud-table
- reflector - by emberstack

All the installation is done using helm charts except vault.
Vauld config are saved in https://develop.vicara.co/ashish.oli/banzaicloud-vault you can clone and make the nesseccary changes required.
You need to change the ingresss in the crd.yaml file to whereever you want to expose it and apply the file which

Certificate, clusterissuer and cloudflare
To configure all these things we have a file for all in them folder which are named `wildcard-cluster-certificate.yaml`, `cluster-issuer.yaml` and `cloudflare-api-token.yaml`

`cloudflare-api-token` can now be put inside cert-manager and don't need to be needing reflector for this anymore like first cluster.

You would be needing a cloudflare apitoken to create a certificate you can get is by going to you cloudflare account and creating a token with the following steps

- Tokens can be created at `User Profile > API Tokens > API Tokens`. The following settings are recommended:
- Permissions:
	Zone - DNS - Edit
	Zone - Zone - Read
- Zone Resources:
	Include - All Zones

To read more on this how it works you can follow the link - `https://cert-manager.io/docs/configuration/acme/dns01/cloudflare/`
To know why do we need these token with certain permission and all those stuff you can read up on `Automated Certificate Management Environment (ACME)`
on official cert-manager docs - `https://cert-manager.io/docs/configuration/acme/`

Generating a certificate might take some time hence monitor it status using describe -> certificate, certificateRequest, orders, challenges

Ingress controller needs the certificate as secret under the name `tls-domain-wildcard-patr-cloud` as a default-ssl-certificate 
