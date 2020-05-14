import express from 'express';
import { getRepoDeployments } from '../../models/database-modules/deployment';
import module from '../../module';

const router = express.Router();

router.use(
	express.json({
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
		});
	}
	return 1;
});

export default router;
