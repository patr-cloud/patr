import { Router } from 'express';

import { createDeployment, getDeploymentById, deleteDeployment } from '../../models/database-modules/deployment';
import { errors, messages } from '../../config/errors';
import {
	deleteSSL, generateVerification, deleteNginxConfig, verifyDomain,
	cleanupChallenge, generateProxy, generateSSL,
} from './nginx';
import {
	deleteDomain, getDomain, createDomain, verifyDomainDB, getDeploymentDomains,
} from '../../models/database-modules/domain';
import check from './middleware';
import { permissions } from '../../models/interfaces/permission';
import {domainRegex} from '../../config/constants';

const router = Router();

router.post('/:groupName/deployment', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.create, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.repository || !req.body.tag || !req.body.configuration || !req.body.serverId) {
		return res.status(400).json({
			success: false,
		});
	}

	delete req.body.configuration.HostConfig;

	await createDeployment({
		deploymentId: null,
		repository: req.body.repository,
		tag: req.body.tag,
		configuration: req.body.configuration,
		serverId: req.body.serverId,
	});

	return res.json({
		success: true,
	});
});

router.delete('/:groupName/deployment/:deploymentId', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.delete, resourceName)(req, res, next);
}, async (req, res, next) => {
	const domains = await getDeploymentDomains(
		Buffer.from(req.params.deploymentId, 'hex'),
	);
	// For each domain directly linked to the deployment
	// remove its nginx config and ssl certificates
	await Promise.all(domains.map((domain) => {
		const tasks = [deleteNginxConfig(domain.domain)];
		if (domain.verified === 1) {
			tasks.push(deleteSSL(domain.domain));
		}
		return Promise.all(tasks);
	}));
	await deleteDeployment(Buffer.from(req.params.deploymentId, 'hex'));
	return res.json({
		success: true,
	});
});

// Configure a new domain for a deployment
router.post('/:groupName/domain', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.addDomain, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.domain || !req.body.deploymentId || !req.body.port) {
		return res.status(400).json({
			success: false,
		});
	}

	if (!(req.body.domain as string).match(domainRegex)) {
		return res.status(400).json({
			success: false,
		});
	}

	// Check if domain already mapped
	if (getDomain(req.body.domain)) {
		return res.status(400).json({
			success: false,
		});
	}
	// TODO: Handle load balancing multiple deployments here later, for now just pick first deployment
	const deployment = await getDeploymentById(
		Buffer.from(req.params.deploymentId, 'hex'),
	);

	// Check if deployment is actually exposing port. If it is, get the machine port
	const machinePort = deployment.configuration?.HostConfig?.PortBindings[req.body.port]?.HostPort;

	if (!machinePort) {
		return res.json({
			success: false,
			error: errors.portNotExposed,
			nessages: messages.portNotExposed,
		});
	}

	// Setup a temporary nginx config for domain verification
	const challenge = await generateVerification();
	await createDomain({
		deploymentId: req.body.deploymentId,
		domain: req.body.domain,
		port: req.body.port,
		verified: 0,
		challenge,
	});
	return res.json({
		success: true,
	});
});


router.post('/:groupName/domain/verify', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.verifyDomain, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.domain) {
		return res.status(400).json({
			success: false,
		});
	}

	const domain = await getDomain(req.body.domain);

	if (!domain) {
		return res.status(400).json({
			success: false,
		});
	}

	if (domain.verified === 1) {
		return res.json({
			success: true,
			verified: true,
		});
	}

	const verified = await verifyDomain(domain.domain, domain.challenge);

	if (verified) {
		// Mark the domain as verified in the database, cleanup the challenge,
		// and setup the ssl for the domain
		const deployment = await getDeploymentById(domain.deploymentId);
		await Promise.all([
			verifyDomainDB(domain.domain),
			cleanupChallenge(domain.challenge),
			generateSSL(domain.domain),
		]);
		await generateProxy(domain.domain, deployment.ip, domain.port);
	}

	return res.json({
		success: true,
		verified,
	});
});

// Delete a configured domain
router.delete('/:groupName/domain', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.removeDomain, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.domain) {
		return res.status(400).json({
			success: false,
		});
	}

	const domain = await getDomain(req.body.domain);
	if (!domain) {
		return res.status(400).json({
			success: false,
		});
	}

	const tasks = [];

	tasks.push(deleteNginxConfig(req.body.domain));

	if (domain.verified === 1) {
		tasks.push(deleteSSL(req.body.domain));
	}

	tasks.push(deleteDomain(req.body.domain));
	await Promise.all(tasks);
	return res.json({
		success: true,
	});
});

export default router;
