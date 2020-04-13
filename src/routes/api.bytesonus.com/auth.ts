import { Router } from 'express';
import { compare, hash } from 'bcrypt';
import { encode } from 'jwt-handler';
import { v4 } from 'uuid';
import { generate } from 'randomstring';
import { createTransport } from 'nodemailer';
import {
	getUserByUsernameOrEmail,
	addNewLogin,
	getLoginForAuthToken,
	setAuthTokenExpiry,
	addEmailInvite,
	getInviteForEmail,
	deleteInvite,
	addUser,
	getUserByEmail,
	createPasswordResetRequest,
	getPasswordResetRequestForToken,
	updateUserPassword,
	deletePasswordResetRequest,
	getGroupsForUser,
	isEmailAvailable,
	isUsernameAvailable,
	getUserDetails
} from '../../models/database';
import { errors, messages } from '../../config/errors';
import { email, jwtSecret, passwordSaltRounds } from '../../config/config';
import { isEmailValid, isUsernameValid, isPasswordValid } from '../../models/interfaces/validators';
import { AccessTokenData } from '../../models/interfaces/access-token-data';
import { createHash } from 'crypto';

const router = Router();

const transporter = createTransport({
	host: email.host,
	port: email.port,
	secure: email.secure,
	auth: {
		user: email.username,
		pass: email.password
	}
});

router.post('/signIn', async (req, res) => {
	if (!req.body.userId) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	const user = await getUserByUsernameOrEmail(req.body.userId);
	if (user == null) {
		res.json({
			success: false,
			error: errors.userNotFound,
			message: messages.userNotFound
		});
		return;
	}

	const success = await compare(req.body.password, user.password);
	if (success !== true) {
		res.json({
			success: false,
			error: errors.invalidPassword,
			message: messages.invalidPassword
		});
		return;
	}
	const userId = user.userId;

	const groups = await getGroupsForUser(userId);

	delete user.userId;
	delete user.password;
	delete user.created;

	const authToken = v4();
	const accessObject = new AccessTokenData();
	accessObject.user = user;
	accessObject.groups = groups;
	const accessToken = await accessObject.toJwtString();

	res.json({
		success: true,
		data: {
			authToken: authToken,
			accessToken: accessToken
		}
	});

	await addNewLogin(userId, authToken, accessObject.exp);
});

router.get('/accessToken', async (req, res) => {
	if (!req.headers.authorization) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	const login = await getLoginForAuthToken(req.headers.authorization);
	if (login == null) {
		res.status(401).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	} else if (login.authExp <= Date.now()) {
		res.status(401).json({
			success: false,
			error: errors.unauthorized,
			message: messages.unauthorized
		});
		return;
	}

	const userId = login.userId;

	const groups = await getGroupsForUser(userId);
	const userData = await getUserDetails(userId);

	const accessObject = new AccessTokenData();
	accessObject.user = userData;
	accessObject.groups = groups;
	const accessToken = await accessObject.toJwtString();

	res.json({
		success: true,
		data: {
			accessToken: accessToken
		}
	});

	await setAuthTokenExpiry(req.headers.authorization, accessObject.exp);
});

router.post('/signUp', async (req, res) => {
	if (!req.body.email) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}
	if (!req.body.username) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}
	if (!req.body.password) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	if (isEmailValid(req.body.email) !== true) {
		res.json({
			success: false,
			error: errors.invalidEmail,
			message: messages.invalidEmail
		});
		return;
	}
	const emailAvailable = await isEmailAvailable(req.body.email);
	if (emailAvailable !== true) {
		res.json({
			success: false,
			error: errors.emailTaken,
			message: messages.emailTaken
		});
		return;
	}

	if (isUsernameValid(req.body.username) !== true) {
		res.json({
			success: false,
			error: errors.invalidUsername,
			message: messages.invalidUsername
		});
		return;
	}
	const usernameAvailable = await isUsernameAvailable(req.body.username);
	if (usernameAvailable !== true) {
		res.json({
			success: false,
			error: errors.usernameTaken,
			message: messages.usernameTaken
		});
		return;
	}

	if (isPasswordValid(req.body.password) !== true) {
		res.json({
			success: false,
			error: errors.passwordTooWeak,
			message: messages.passwordTooWeak
		});
		return;
	}

	const token = generate({
		length: 40,
		charset: 'alphabetic'
	});
	const tokenExpiry = Date.now() + (1000 * 60 * 60 * 24); // 24 hours
	const password = await hash(req.body.password, passwordSaltRounds);

	const tokenHash = createHash('sha256')
		.update(token)
		.digest('hex');
	await addEmailInvite(req.body.email, req.body.username, password, tokenHash, tokenExpiry);

	res.json({
		success: true
	});

	await transporter.sendMail({
		from: email.from,
		to: req.body.email,
		subject: 'Verify your account | bytesonus',
		html: `Verify your account plizz. <br>
		Here's a simple link to use for the token:
			<a href="https://accounts.bytesonus.com/auth/verify?token=${token}&email=${req.body.email}">
				https://accounts.bytesonus.com/auth/verify?token=${token}&email=${req.body.email}
			</a>
		<br>
		This token expires in 24 hours.<br>
		TODO: Proper email goes here, with HTML templates and all`
	});
});

router.get('/emailAvailable', async (req, res) => {
	if (!req.query.email) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	if (isEmailValid(req.query.email) !== true) {
		res.json({
			success: false,
			error: errors.invalidEmail,
			message: messages.invalidEmail
		});
		return;
	}
	const emailAvailable = await isEmailAvailable(req.query.email);
	if (emailAvailable !== true) {
		res.json({
			success: false,
			error: errors.emailTaken,
			message: messages.emailTaken
		});
		return;
	}

	res.json({
		success: true
	});
});

router.get('/usernameAvailable', async (req, res) => {
	if (!req.query.username) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	if (isUsernameValid(req.query.username) !== true) {
		res.json({
			success: false,
			error: errors.invalidUsername,
			message: messages.invalidUsername
		});
		return;
	}
	const usernameAvailable = await isUsernameAvailable(req.query.username);
	if (usernameAvailable !== true) {
		res.json({
			success: false,
			error: errors.usernameTaken,
			message: messages.usernameTaken
		});
		return;
	}

	res.json({
		success: true
	});
});

router.post('/join', async (req, res) => {
	if (!req.body.token) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}
	if (!req.body.email) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	const tokenHash = createHash('sha256')
		.update(req.body.token)
		.digest('hex');
	const invite = await getInviteForEmail(req.body.email);
	if (invite == null) {
		res.json({
			success: false,
			error: errors.tokenNotFound,
			message: messages.tokenNotFound
		});
		return;
	} else if (invite.token !== tokenHash) {
		res.json({
			success: false,
			error: errors.tokenNotFound,
			message: messages.tokenNotFound
		});
		return;
	} else if (invite.tokenExpiry <= Date.now()) {
		res.json({
			success: false,
			error: errors.tokenExpired,
			message: messages.tokenExpired
		});
		return;
	}

	await addUser(invite.username, invite.email, invite.password);
	await deleteInvite(invite);

	res.json({
		success: true
	});
});

router.post('/forgotPassword', async (req, res) => {
	if (!req.body.email) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	const user = await getUserByEmail(req.body.email);
	if (user == null) {
		res.json({
			success: false,
			error: errors.invalidEmail,
			message: messages.invalidEmail
		});
		return;
	}
	const userId = user.userId;

	const token = generate({
		length: 20,
		charset: 'alphabetic'
	});
	const tokenExpiry = Date.now() + (1000 * 60 * 60 * 24); // 24 hours

	await createPasswordResetRequest(userId, token, tokenExpiry);

	res.json({
		success: true
	});

	await transporter.sendMail({
		from: email.from,
		to: req.body.email,
		subject: 'Password reset request | bytesonus',
		html: `Did you want to reset password boi?<br>
		Here, tek link to reset:
			<a href="https://accounts.bytesonus.com/auth/reset-password?token=${token}&email=${req.body.email}">
				https://accounts.bytesonus.com/auth/reset-password?token=${token}&email=${req.body.email}
			</a>
		<br>
		TODO: HTML templates for emails and all`
	});
});

router.post('/resetPassword', async (req, res) => {
	if (!req.body.token) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}
	if (!req.body.password) {
		res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters
		});
		return;
	}

	const passwordResetRequest = await getPasswordResetRequestForToken(req.body.token);
	if (passwordResetRequest == null) {
		res.status(401).json({
			success: false,
			error: errors.tokenNotFound,
			message: messages.tokenNotFound
		});
		return;
	}

	if (passwordResetRequest.tokenExpiry <= Date.now()) {
		res.status(401).json({
			success: false,
			error: errors.tokenExpired,
			message: messages.tokenExpired
		});
		return;
	}

	const passwordHash = await hash(req.body.password, passwordSaltRounds);
	await updateUserPassword(passwordResetRequest.userId, passwordHash);
	await deletePasswordResetRequest(passwordResetRequest);

	res.json({
		success: true
	});

	await transporter.sendMail({
		from: email.from,
		to: req.body.email,
		subject: 'Password has been reset | bytesonus',
		html: 'Bruv, your password has been reset'
	});
});

export default router;
