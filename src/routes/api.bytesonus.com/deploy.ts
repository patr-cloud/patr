import { Router } from 'express';
import { lookup } from 'dns';
import { promisify } from 'util';
import { writeFile } from 'fs-extra-promise';
import { join } from 'path';

import { createDeployment, getDeploymentsById } from '../../models/database-modules/deployment';
import { errors, messages } from '../../config/errors';
import { generateNginxConfig, generateSSL, deleteSSL } from './nginx';
import { nginxFolder } from '../../config/config';
import getJunoModule from '../../module';
import { deleteDomain, getDomain, createDomain } from '../../models/database-modules/domain';
import check from './middleware';
import { permissions } from '../../models/interfaces/permission';

const lookupPromise = promisify(lookup);

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

// Configure a new domain for a deployment
router.post('/::groupName/deployment/::deploymentId/domain', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.addDomain, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.domain || !req.params.deploymentId || !req.body.port) {
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
	// TODO: Regex for domain name

	// TODO: Handle load balancing multiple deployments here later, for now just pick first deployment
	const deployments = (await getDeploymentsById(req.body.deploymentId))[0];

	// Check if deployment is actually exposing port. If it is, get the machine port
	const machinePort = deployments.configuration?.HostConfig?.PortBindings[req.body.port]?.HostPort;

	if (!machinePort) {
		return res.json({
			success: false,
			error: errors.portNotExposed,
			nessages: messages.portNotExposed,
		});
	}
	// DNS lookup
	const ip = await lookupPromise(req.body.domain);

	if (deployments.ip !== ip.address) {
		return res.json({
			success: false,
			error: errors.dnsNotConfigured,
			message: messages.dnsNotConfigured,
		});
	}

	try {
		await generateSSL(req.body.domain);
	} catch {
		res.json({
			success: false,
			error: errors.sslGenerationFailed,
			message: messages.sslGenerationFailed,
		});
	}

	const nginxConfig = generateNginxConfig(req.body.domain, deployments.ip, machinePort);

	await writeFile(join(nginxFolder, req.body.domain), nginxConfig);
	const module = await getJunoModule();
	module.triggerHook('reload');

	await createDomain({
		deploymentId: req.body.deploymentId,
		domain: req.body.domain,
		port: req.body.port,
	});
	return res.json({
		success: true,
	});
});

// Delete a configured domain
router.delete('/::groupName/deployment/::deploymentId/domain', async (req, res, next) => {
	const resourceName = `${req.params.groupName}::deployer`;
	return check(permissions.Deployer.removeDomain, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.domain) {
		return res.status(400).json({
			success: false,
		});
	}


	if (!getDomain(req.body.domain)) {
		return res.status(400).json({
			success: false,
		});
	}

	await deleteSSL(req.body.domain);
	await deleteDomain(req.body.domain);
	return res.json({
		success: true,
	});
});

export default router;
