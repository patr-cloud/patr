import { Router } from 'express';
import { createDeployment, setDeploymentServers } from '../../models/database-modules/deployment';

const router = Router();

// TODO: Permission checks,only group owner can do this
router.post('/new', async (req, res, next) => {
	if (!req.body.repository || !req.body.tag || !req.body.configuration) {
		return res.status(400).json({
			success: false,
		});
	}

	await createDeployment(
		req.body.repository,
		req.body.tag,
		req.body.configuration,
	);

	return res.json({
		success: true,
	});
});

// Update servers to which a container deploys
router.post('/:id/servers', async (req, res, next) => {
	if (!req.body.servers) {
		return res.status(400).json({
			success: false,
		});
	}

	await setDeploymentServers(req.params.id, req.body.servers);

	return res.json({
		success: true,
	});
});

// Configure a new domain for a deployment
router.post('/domain', async (req, res, next) => {
	if (!req.body.domain || (!req.body.redirect && !req.body.deploymentId)) {
		return res.status(400).json({
			success: false,
		});
	}

	// TODO:
	// 1. Check if domain is a valid domain name, with DNS Records pointing to the deployment server
	// 2. Setup nginx configurations and write to config.NGINX_CONFIGS_PATH
});

export default router;
