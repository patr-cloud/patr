import { Router } from 'express';
import { lookup } from 'dns';
import { promisify } from 'util';
import { writeFile } from 'fs-extra-promise';
import { join } from 'path';

import { createDeployment, getDeploymentsById } from '../../models/database-modules/deployment';
import { errors, messages } from '../../config/errors';
import { generateNginxConfig, generateSSL, deleteSSL } from './nginx';
import { nginxFolder } from '../../config/config';
import module from '../../module';
import { deleteDomain, getDomain, createDomain } from '../../models/database-modules/domain';

const lookupPromise = promisify(lookup);

const router = Router();

// TODO: Permission checks,only group owner can do this
router.post('/new', async (req, res, next) => {
	if (!req.body.repository || !req.body.tag || !req.body.configuration || !req.body.serverId) {
		return res.status(400).json({
			success: false,
		});
	}

	await createDeployment(
		req.body.repository,
		req.body.tag,
		req.body.configuration,
		req.body.serverId,
	);

	return res.json({
		success: true,
	});
});

// Configure a new domain for a deployment
router.post('/domain', async (req, res, next) => {
	if (!req.body.domain || !req.body.deploymentId || !req.body.port) {
		return res.status(400).json({
			success: false,
		});
	}

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
	module.triggerHook('reload');

	await createDomain(req.body.deploymentId, req.body.domain, req.body.port);
	return res.json({
		success: true,
	});
});

// Delete a configured domain
router.delete('/domain', async (req, res, next) => {
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
