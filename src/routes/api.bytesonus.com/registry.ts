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
			Object.values(deployments).forEach((deployment: any) => {
				module.callFunction('deployer.deploy', deployment);
			});
		}
	});

	res.json({ success: true });
});

export default router;
