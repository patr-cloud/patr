import { JSONWebKeySet } from 'jose';
import { readFileSync } from 'fs';
import { join } from 'path';

import { RegAuth } from '../models/interfaces/deployment';

export const port = '5001';
export const basePath = '/';
export const saltRounds = 10;
export const jwtSecret = 'foobar';

export const mysql = {
	host: 'localhost',
	port: 3306,
	user: 'bytesonus',
	password: 'bytesonus',
	database: 'bytesonus',
	connectionLimit: 10,
};

export const cookieKeys = ['secretkey'];
export const jwks: JSONWebKeySet = JSON.parse(readFileSync(join(__dirname, 'jwks.json')).toString());

export const redis = 'redis://localhost:6379/1';
export const nginxFolder = '/etc/nginx/sites_enabled';
export const certsFolder = '/etc/letsencrypt/live';
export const certbotWebRoot = '/var/www/letsencrypt';
export const bytesonusWebRoot = '/var/www/bytesonus';

export const registryUrl = 'registry.bytesonus.com:5001';

export const dockerHubRegistry: RegAuth = {
	serveraddress: 'registry.hub.docker.com',
	username: '',
	password: '',
};

export const privateRegistry: RegAuth = {
	serveraddress: registryUrl,
	username: 'bytesonus',
	password: 'bytesonus',
};

export const registryPrivateKey = readFileSync(join(__dirname, 'example.org.key'));
export const registryPublicKeyDER = readFileSync(join(__dirname, 'example.org.pubkey.der'));

export const apiDomain = 'api.bytesonus.com:5001';
export const authDomain = 'auth.bytesonus.com:5001';
