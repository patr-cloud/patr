import { Router, json } from 'express';
import { createHash, randomBytes } from 'crypto';
import base32Encode from 'base32-encode';
import { JWK, JWT } from 'jose';
import { ContainerCreateOptions, HostConfig } from 'dockerode';

import {
	registryPrivateKey, registryPublicKeyDER, registryUrl, apiDomain,
} from '../../config/config';
import { getRepoDeployments, updateDeploymentConfig } from '../../models/database-modules/deployment';
import getJunoModule from '../../module';
import { errors, messages } from '../../config/errors';
import { getUserByUsername } from '../../models/database-modules/user';
import Account from '../auth.bytesonus.com/oidc/account';
import { RegistryClaims } from '../../models/interfaces/registry';
import { permissions } from '../../models/interfaces/permission';
import { getUserOrgs } from '../../models/database-modules/organization';
import checkIfUserHasPermission from '../../models/database-modules/permission';
import { User } from '../../models/interfaces/user';
import { DeployJob } from '../../models/interfaces/deployment';


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
function generateKid(publicKey: Buffer) {
	const encoded = base32Encode(
		createHash('sha256').update(publicKey).digest().slice(0, 30),
		'RFC4648',
	);

	const chunks = [];
	for (let i = 0; i < encoded.length; i += 4) {
		chunks.push(encoded.substr(i, 4));
	}

	return chunks.join(':');
}

const jwtSigningKey = JWK.asKey(registryPrivateKey, {
	kid: generateKid(registryPublicKeyDER),
	alg: 'RS256',
});


/*
 * Checks is a user (belonging to userGroups) is permitted
 * to perform the actions specified by the given docker
 * registry scopes.
 *
 * Returns the claims that were granted, in the format specified
 * in https://docs.docker.com/registry/spec/auth/jwt/
 *
* */
async function grantedClaims(
	user: User,
	userOrgs: Buffer[],
	scopes: string[],
): Promise<RegistryClaims> {
	const access: RegistryClaims = await Promise.all(scopes.map(async (scope) => {
		const [_type, repository, actions] = scope.split(':');
		const [org, image] = repository.split('/');

		if (!image) {
			// User tried to push without a group name, access
			// not granted
			return {
				type: 'repository' as const,
				name: repository,
				actions: [],
			};
		}

		const actionsList = actions.split(',') as ('push' | 'pull')[];
		const permsRequested = actionsList.map((action) => {
			if (action === 'push') {
				return permissions.DockerRegistry.push;
			}
			if (action === 'pull') {
				return permissions.DockerRegistry.pull;
			}
			throw Error(`Unkown action ${action} requested by registry!`);
		});

		const granted = await checkIfUserHasPermission(
			user.userId,
			userOrgs,
			`${org}::docker_registry`,
			permsRequested,
		);

		return {
			type: 'repository' as const,
			name: repository,
			actions: actionsList.filter((_action, i) => granted[i]),
		};
	}));
	return access;
}

const router = Router();
router.use(
	json({
		type: 'application/vnd.docker.distribution.events.v1+json',
	}),
);

/**
 * Takes an array of deployer jobs, and triggers deployer.deploy
 * The host config returned back by deployer is updated in the
 * database
 */
export async function deploy(jobs: DeployJob[]) {
	const module = await getJunoModule();
	const containers = await module.callFunction('deployer.deploy', {
		jobs,
	});
	await Promise.all(containers.map(
		(container: { id: string, configuration: HostConfig }) => {
			const hostConfig = container.configuration;
			return updateDeploymentConfig(
				Buffer.from(container.id, 'hex'),
				hostConfig,
			);
		},
	));
}

/**
 * Route to deploy docker container
 */
router.get('/event', async (req, res) => {
	req.body.events.map(async (event: any) => {
		if (
			event.action === 'push'
			&& event.target.mediaType
			=== 'application/vnd.docker.distribution.manifest.v2+json'
		) {
			const { tag } = event.target;
			const repo = event.target.repository;
			const deployments = await getRepoDeployments(repo, tag);
			deploy(deployments);
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
	if (!req.query.scope) {
		scopes = [];
	}

	// If a single scope is passed, convert it to array
	if (typeof req.query.scope === 'string') {
		scopes = [req.query.scope];
	} else {
		scopes = req.query.scope as string[];
	}

	const userOrgs = (await getUserOrgs(user.userId)).map((g) => g.organizationId);

	const token = JWT.sign({
		access: await grantedClaims(user, userOrgs, scopes),
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
