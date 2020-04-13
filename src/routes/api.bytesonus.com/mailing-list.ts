import { Router } from 'express';
import HashSet = require('hashset');
import { readFileSync, appendFileAsync, existsSync, createFileSync } from 'fs-extra-promise';
import { errors, messages } from '../../config/errors';
import { isEmailValid } from '../../models/interfaces/validators';

const router = Router();

const mailingList = new HashSet();

if (existsSync('../emails.txt')) {
	readFileSync('../emails.txt')
		.toString()
		.split('\n')
		.forEach(email => {
			mailingList.add(email);
		});
} else {
	createFileSync('../emails.txt');
}

router.post('/subscribe', async (req, res) => {
	if (!req.body.email) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	if (!isEmailValid(req.body.email)) {
		res.json({
			success: false,
			error: errors.invalidEmail,
			message: messages.invalidEmail
		});
		return;
	}

	if (mailingList.contains(req.body.email) !== true) {
		await appendFileAsync('../emails.txt', req.body.email + '\n');
		mailingList.add(req.body.email);
	}

	res.json({
		success: true
	});
});

export default router;
