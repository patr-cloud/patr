import { Router } from 'express';
import { createDeployment } from '../../models/database-modules/deployment';

const router = Router();

// TODO: Permission checks,only group owner can do this
router.post('/new', async (req, res, next) => {
	if (!req.body.repository || !req.body.tag || !req.body.serverId || !req.body.configuration) {
		return res.status(400).json({
			success: false,
		});
	}

	await createDeployment(
		req.body.repository,
		req.body.tag,
		req.body.serverId,
		req.body.configuration,
	);

	return res.json({
		success: true,
	});
});

export default router;
