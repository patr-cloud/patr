import { exec } from 'child_process';
import { promisify } from 'util';

const execPromise = promisify(exec);

export function generateNginxConfig(domain: string, ip: string, port: string | number) {
	return `
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
}

export async function generateSSL(domain: string) {
	await execPromise(`certbot certonly --webroot -w /var/www/example -d ${domain}`);
}
