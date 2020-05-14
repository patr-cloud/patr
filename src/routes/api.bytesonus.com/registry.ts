import { Router, json } from 'express';
import { createHash, randomBytes } from 'crypto';
import base32Encode from 'base32-encode';
import { JWK, JWT } from 'jose';

import {
	registryPrivateKey, registryPublicKeyDER, registryUrl, apiDomain,
} from '../../config/config';
import { getRepoDeployments } from '../../models/database-modules/deployment';
import getJunoModule from '../../module';
import { errors, messages } from '../../config/errors';
import { getUserByUsername } from '../../models/database-modules/user';
import Account from '../auth.bytesonus.com/oidc/account';


/*
 * Utility function to generate the JWT Kid as per docker registry
 * requirements
 *
 * From https://docs.docker.com/registry/spec/auth/jwt/ :
 *
 * 1. Take the DER encoded public key which the JWT token was signed against.
 * 2. Create a SHA256 hash out of it and truncate to 240bits.
 * 3. Split the result into 12 base32 encoded groups with : as delimiter.
 *
 * Idk why docker can't just follow the jwk standard
* */
const generateKid = (publicKey: Buffer) => {
	const encoded = base32Encode(
		createHash('sha256').update(publicKey).digest().slice(0, 30),
		'RFC4648',
	);

	const chunks = [];
	for (let i = 0; i < encoded.length; i += 4) {
		chunks.push(encoded.substr(i, 4));
	}

	return chunks.join(':');
};

const jwtSigningKey = JWK.asKey(registryPrivateKey, {
	kid: generateKid(registryPublicKeyDER),
	alg: 'RS256',
});

const router = Router();

router.use(
	json({
		type: 'application/vnd.docker.distribution.events.v1+json',
	}),
);

router.post('/event', async (req, res) => {
	req.body.events.map(async (event: any) => {
		if (
			event.action === 'push'
			&& event.target.mediaType
			=== 'application/vnd.docker.distribution.manifest.v2+json'
		) {
			const { tag } = event.target;
			const repo = event.target.repository;
			const deployments = await getRepoDeployments(repo, tag);
			const module = await getJunoModule();
			module.callFunction('deployer.deploy', deployments);
		}
	});

	res.json({ success: true });
});


/**
 * Route to provide docker registry with
 * tokens. Checks if user has access to perform
 * the requested action on the resource, and grants
 * only those permissions which were allowed.
 */
router.get('/token', async (req, res) => {
	if (!req.headers.authorization) {
		return res.status(401).json({
			success: false,
			error: errors.unauthorized,
			message: messages.unauthorized,
		});
	}

	// Extract username and password from HTTP basic auth header
	const credentials = Buffer.from(
		req.headers.authorization.split(' ')[1] || ' ',
		'base64',
	).toString();

	const splitIndex = credentials.indexOf(':');

	const username = credentials.substring(0, splitIndex);
	const password = credentials.substring(splitIndex + 1);

	if (!username || !password) {
		res.status(401).json({
			success: false,
			errors: errors.unauthorized,
			message: messages.unauthorized,
		});
	}

	const user = await getUserByUsername(username);

	if (!user || !(await Account.authenticate(username, password))) {
		return res.status(401).json({
			success: false,
			errors: errors.unauthorized,
			message: messages.unauthorized,
		});
	}

	/*
	 * Scopes contains the actions the registry wants to perform, for example,
	 * repository:samalba/my-app:pull,push
	 * is requesting pull, and push access on the image samalba/my-app
	 * */
	let scopes: string[];

	// Scopes is not passed in the case of docker login
	if (!req.query.scopes) {
		scopes = [];
	}

	// If a single scope is passed, convert it to array
	if (typeof req.query.scopes === 'string') {
		scopes = [req.query.scopes];
	}

	const token = JWT.sign({
		access: [],
	},
	jwtSigningKey,
	{
		algorithm: 'RS256',
		issuer: apiDomain,
		subject: username,
		expiresIn: '10 m',
		jti: randomBytes(256).toString(),
		notBefore: '0 s',
		audience: registryUrl,
	});
	return res.json({
		token,
	});
});

export default router;
