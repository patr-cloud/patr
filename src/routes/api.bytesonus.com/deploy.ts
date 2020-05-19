import { Router } from 'express';
import { v4 } from 'uuid';
import path from 'path';
import url from 'url';
import { ContainerCreateOptions } from 'dockerode';

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
import { domainRegex, volumesDir } from '../../config/constants';
import { getOrganizationByName } from '../../models/database-modules/organization';
import { getServerById } from '../../models/database-modules/server';
import checkIfUserHasPermission from '../../models/database-modules/permission';
import { dockerHubRegistry } from '../../config/config';
import { deploy } from './registry';

const parseBindings = (binds: ContainerCreateOptions['HostConfig']['PortBindings']) => Object.keys(binds).every((containerPort) => {
	if (Array.isArray(binds[containerPort]) && binds[containerPort].length === 0) {
		binds[containerPort] = [{
			HostIp: '0.0.0.0',
			HostPort: '',
		}];
		return true;
	} return false;
});

const parseMounts = (mounts: ContainerCreateOptions['HostConfig']['Mounts']) => mounts.every((mount) => {
	if (!mount.Source) {
		return true;
	} return false;
});

const bindVolumeSource = (mounts: ContainerCreateOptions['HostConfig']['Mounts'], deploymentId: string) => mounts.map((mount) => {
	const volumeUUID = v4();
	mount.Source = path.join(volumesDir, deploymentId, volumeUUID);
	return mount;
});


const router = Router();

router.post('/:orgName/deployment', async (req, res, next) => {
	const resourceName = `${req.params.orgName}::deployer`;
	return check(permissions.Deployer.create, resourceName)(req, res, next);
}, async (req, res, next) => {
	if (!req.body.repository || !req.body.tag || !req.body.configuration || !req.body.serverId) {
		return res.status(400).json({
			success: false,
		});
	}

	const serverId = Buffer.from(req.body.serverId, 'hex');

	const server = await getServerById(serverId);

	if (!server) {
		return res.json({
			success: false,
		});
	}
	const { PortBindings, Mounts, ...rest } = req.body.configuration.HostConfig;
	const deploymentId = v4({}, Buffer.alloc(16));

	// Check if only PortBindings and Mounts are passed in HostConfig
	if (Object.keys(rest).length !== 0) {
		return res.json({
			success: false,
			error: errors.invalidHostConfig,
			messages: messages.invalidHostConfig,
		});
	}
	// Check if no host port is mapped to containerPorts inside PortBinding
	if (PortBindings && !parseBindings(PortBindings)) {
		return res.json({
			success: false,
			error: errors.invalidPortBindings,
			messages: messages.invalidPortBindings,
		});
	}
	// Check if no machine path is mapped to containerPath inside Mounts
	if (Mounts && !parseMounts(Mounts)) {
		return res.json({
			success: false,
			error: errors.invalidPortBindings,
			messages: messages.invalidPortBindings,
		});
	}
	// Allow only filtered paths and mounts to be passed through HostConfig
	if (Mounts) {
		const bindedMounts = bindVolumeSource(Mounts, deploymentId.toString());
		req.body.configuration.HostConfig.Mounts = bindedMounts;
	}

	if (PortBindings) {
		req.body.configuration.HostConfig.PortBindings = PortBindings;
	}

	const organization = await getOrganizationByName(req.params.orgName);

	const deployment = await createDeployment({
		deploymentId,
		repository: req.body.repository,
		tag: req.body.tag,
		configuration: req.body.configuration,
		serverId,
		organizationId: organization.organizationId,
	});

	const hostname = req.body.repository.slice(0, req.body.repository.indexOf('/'));
	// Docker hub images can be deployed directly, instead of triggering only
	// on a push to the registry.
	// TODO: Should the same behvaious happen for existing images already pushed on the
	// private registry?
	if (hostname === dockerHubRegistry.serveraddress) {
		deploy([{
			id: deployment.deploymentId.toString('hex'),
			image: `${deployment.repository}:${deployment.tag}`,
			server: {
				host: server.host,
				port: server.port,
			},
			configuration: req.body.configuration,
		}]);
	}

	return res.json({
		success: true,
	});
});

router.delete('/:orgName/deployment/:deploymentId', async (req, res, next) => {
	const resourceName = `${req.params.orgName}::deployer`;
	return check(permissions.Deployer.delete, resourceName)(req, res, next);
}, async (req, res, next) => {
	const [deployment, organization] = await Promise.all([
		getDeploymentById(Buffer.from(req.params.deploymentId, 'hex')),
		getOrganizationByName(req.params.orgName),
	]);

	if (!deployment.organizationId.equals(organization.organizationId)) {
		return res.status(400).json({
			success: false,
		});
	}
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
router.post('/:orgName/domain', async (req, res, next) => {
	const resourceName = `${req.params.orgName}::deployer`;
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
	const machinePort = deployment.hostConfig?.PortBindings[req.body.port]?.HostPort;

	if (!machinePort) {
		return res.json({
			success: false,
			error: errors.portNotExposed,
			messages: messages.portNotExposed,
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


router.post('/:orgName/domain/verify', async (req, res, next) => {
	const resourceName = `${req.params.orgName}::deployer`;
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
router.delete('/:orgName/domain', async (req, res, next) => {
	const resourceName = `${req.params.orgName}::deployer`;
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
