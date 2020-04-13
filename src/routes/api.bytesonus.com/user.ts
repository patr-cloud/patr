import { Request, Router } from 'express';
import { s3 as s3Config } from '../../config/config';
import { tokenAuthenticator } from './middlewares';
import { getUserDetails, updateUserDetails } from '../../models/database';
import { messages, errors } from '../../config/errors';
import multer = require('multer');
import multerS3 = require('multer-s3');
import { S3, Endpoint } from 'aws-sdk';
import { compare } from 'bcrypt';

const router = Router();

const s3 = new S3({
	endpoint: s3Config.endpoint,
	accessKeyId: s3Config.key,
	secretAccessKey: s3Config.secret,
});
s3.endpoint = new Endpoint(s3Config.endpoint);

router.get('/info', tokenAuthenticator('*'), async (req, res) => {
	const user = await getUserDetails(req.tokenData.user.userId);

	delete user.userId;
	delete user.password;
	delete user.created;
	delete user.groups;

	res.json({
		success: true,
		data: user
	});
});

router.post('/info', tokenAuthenticator('*'), async (req, res) => {
	// if none of these parameters are sent, reject
	if (
		!req.body.phone &&
		!req.body.dob &&
		!req.body.bio &&
		!req.body.firstName &&
		!req.body.lastName &&
		!req.body.country &&
		!req.body.streetAddress1 &&
		!req.body.streetAddress2 &&
		!req.body.state &&
		!req.body.city &&
		!req.body.pincode
	) {
		res.json({
			success: false,
			error: errors.wrongParameters,
			message: messages[errors.wrongParameters]
		});
		return;
	}

	await updateUserDetails(
		req.tokenData.user.userId,
		req.body.phone,
		req.body.dob,
		req.body.bio,
		req.body.firstName,
		req.body.lastName,
		req.body.country,
		req.body.streetAddress1,
		req.body.streetAddress2,
		req.body.state,
		req.body.city,
		req.body.pincode
	);

	res.json({
		success: true
	});
});

const profilePictureUploader = multer({
	storage: multerS3({
		s3: s3,
		bucket: s3Config.bucket,
		key: (req: Request, _, cb) => {
			const userId = req.tokenData.user.userId;

			const fullPath = `images/profile-pictures/${userId}/original`;
			cb(null, fullPath);
		}
	})
}).single('upload');

router.put('/profile-picture', tokenAuthenticator('*'), profilePictureUploader, async (req, res) => {
	res.json({
		success: true
	});
});

router.post('/changePassword', tokenAuthenticator('*'), async (req, res) => {
	if (!req.body.currentPassword) {
		res.json({
			success: false,
			error: errors.wrongParameters,
			message: messages[errors.wrongParameters]
		});
		return;
	}

	if (!req.body.newPassword) {
		res.json({
			success: false,
			error: errors.wrongParameters,
			message: messages[errors.wrongParameters]
		});
		return;
	}

	const user = await getUserDetails(req.tokenData.user.userId);
	const success = await compare(req.body.password, user.password);
});

export default router;
