import { exec } from 'child_process';
import { promisify } from 'util';
import { v4 } from 'uuid';
import { join } from 'path';
import { resolve } from 'url';
import { writeFile, unlink } from 'fs';
import axios, { AxiosResponse } from 'axios';

import { certbotWebRoot, nginxFolder } from '../../config/config';
import getJunoModule from '../../module';

const execPromise = promisify(exec);
const writeFilePromise = promisify(writeFile);
const unlinkPromise = promisify(unlink);

export async function generateVerification() {
	const challengeId = v4({}, Buffer.alloc(16));
	const challengeStr = challengeId.toString('hex');
	await writeFilePromise(join(certbotWebRoot, `.well-known/acme-challenge/${challengeStr}`), challengeStr);
	return challengeId;
}

export async function verifyDomain(domain: string, challengeId: Buffer) {
	const challengeStr = challengeId.toString('hex');
	let response: AxiosResponse<any>;
	try {
		response = await axios.get(resolve(`http://${domain}`, `/.well-known/acme-challenge/${challengeStr}`));
	} catch (error) {
		return false;
	}
	return response.data === challengeStr;
}

export async function cleanupChallenge(challengeId: Buffer) {
	await unlinkPromise(join(certbotWebRoot, `.well-known/acme-challenge/${challengeId.toString('hex')}`));
}

export async function generateProxy(domain: string, ip: string, port: string | number) {
	const config = `
server {
    listen 80;
    listen [::]:80;
    server_name ${domain};
    
    return 301 https://${domain}$request_uri$is_args$args;
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name ${domain};
    
    ssl_certificate /etc/letsencrypt/live/${domain}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${domain}/privkey.pem;
    
    location / {
        proxy_pass http://${ip}:${port};
    }
    
    include snippets/letsencrypt.conf;
}
`;

	await writeFilePromise(join(nginxFolder, domain), config);

	const module = await getJunoModule();
	module.triggerHook('reload');
}

export async function generateRedirect(fromDomain: string, toDomain: string) {
	const config = `
server {
	listen 80;
	listen [::]:80;
	server_name ${fromDomain}

	return 301 https://${fromDomain}$request_uri$is_args$args;
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name ${fromDomain};
    
    ssl_certificate /etc/letsencrypt/live/${fromDomain}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${fromDomain}/privkey.pem;
    
    location / {
        return 301 https://${toDomain}$request_uri$is_args$args;
    }
    
    include snippets/letsencrypt.conf;
}
	`;

	await writeFilePromise(join(nginxFolder, fromDomain), config);

	const module = await getJunoModule();
	module.triggerHook('reload');
}

export async function deleteNginxConfig(domain: string) {
	await unlinkPromise(join(nginxFolder, domain));
}

export async function generateSSL(domain: string) {
	await execPromise(`certbot certonly --webroot -w ${certbotWebRoot} -d ${domain} --agree-tos --register-unsafely-without-email`);
}

export async function deleteSSL(domain: string) {
	await execPromise(`certbot delete --cert-name ${domain}`);
}
