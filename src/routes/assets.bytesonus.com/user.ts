import { Router } from 'express';
import { s3 as s3Config } from '../../config/config';
import { tokenAuthenticator } from '../api.bytesonus.com/middlewares';
import { S3, Endpoint } from 'aws-sdk';
import { JsonError } from '../../models/json-error';
import { errors } from '../../config/errors';

const router = Router();

const s3 = new S3({
	endpoint: s3Config.endpoint,
	accessKeyId: s3Config.key,
	secretAccessKey: s3Config.secret
});
s3.endpoint = new Endpoint(s3Config.endpoint);

router.get('/profile-picture/:size', tokenAuthenticator('*'), async (req, res, next) => {
	const size = req.params.size;

	s3.getObject({
		Bucket: s3Config.bucket,
		Key: `images/profile-pictures/${req.tokenData.user.userId}/${size}`
	})
		.on('error', (err) => {
			if (err.code === 'NoSuchKey') {
				next(new JsonError(404, errors.resourceDoesNotExist));
				return;
			}

			next(err);
		})
		.createReadStream()
		.pipe(res);
});

router.get('/profile-picture/:userId/:size', async (req, res, next) => {
	const size = req.params.size;
	const userId = req.params.userId;

	s3.getObject({
		Bucket: s3Config.bucket,
		Key: `images/profile-pictures/${userId}/${size}`
	})
		.on('error', (err) => {
			if (err.code === 'NoSuchKey') {
				next(new JsonError(404, errors.resourceDoesNotExist));
				return;
			}

			next(err);
		})
		.createReadStream()
		.pipe(res);
});

export default router;
