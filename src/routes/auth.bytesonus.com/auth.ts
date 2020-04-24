import { Router } from 'express';
import { InteractionResults } from 'oidc-provider';
import { errors, messages } from '../../config/errors';
import Account from './oidc/account';
import oidc from './oidc/provider';
import { getUserByUsername } from '../../models/database-modules/user';
import { sessionCheck, Interaction } from './middleware';

const router = Router();


router.post('/login', sessionCheck, async (req, res, _next) => {
	if (!req.body.username || !req.body.password) {
		return res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters,
		});
	}

	const accountId = await Account.authenticate(req.body.username, req.body.password);
	if (accountId) {
		const result: InteractionResults = {
			login: {
				account: accountId,
				remember: req.body.remember === true,
			},
			consent: { // Grant all scopes/claims as only trusted clients use auth.bytesonus.com
				rejectedScopes: [],
				rejectedClaims: [],
			},
		};

		const redirect = await oidc.interactionResult(req, res, result);
		return res.json({
			success: true,
			redirect,
		});
	}
	return res.json({
		success: false,
		error: errors.invalidCredentials,
		message: messages.invalidCredentials,
	});
});

router.post('/register', sessionCheck, async (req, res, _next) => {
	if (!req.body.username || !req.body.email || !req.body.password) {
		return res.status(400).json({
			success: false,
			error: errors.wrongParameters,
			message: messages.wrongParameters,
		});
	}

	try {
		await Account.register(req.body.username, req.body.email, req.body.password);
		// Redirect back to interaction
		return res.json({
			success: true,
			redirect: '/',
		});
	} catch (e) {
		if (e.message.startsWith('ER_DUP_ENTRY')) {
			return res.json({
				success: false,
				error: errors.duplicateUser,
				message: messages.duplicateUser,
			});
		}
		return res.json({
			success: false,
			error: errors.serverError,
		});
	}
});

router.post('/select_account', sessionCheck, async (req, res, _next) => {
	const { session } = res.locals.interaction as Interaction;
	if (!session) {
		return oidc.interactionFinished(req, res,
			{ select_account: {} },
			{ mergeWithLastSubmission: false });
	}

	const user = await getUserByUsername(session.accountId as string);
	return res.render('select_account', {
		title: 'Continue',
		email: user.email,
		username: user.username,
	});
});


router.get('/register', sessionCheck, async (_req, res, _next) => {
	res.render('pages/register', {
		title: 'Register',
	});
});

router.get('/login', sessionCheck, async (_req, res, _next) => {
	res.render('pages/login', {
		title: 'Login',
	});
});

router.get('/', async (req, res, _next) => {
	const interaction = await oidc.interactionDetails(req, res);
	switch (interaction.prompt.name) {
		case 'select_account':
			res.redirect('/oauth/select_account');
			break;
		case 'login':
			res.redirect('/oauth/login');
			break;
		case 'register':
			res.redirect('/oauth/register');
			break;
		default:
			res.json({
				success: false,
				error: errors.serverError,
				message: messages.serverError,
			});
			break;
	}
});

router.use(oidc.callback);

export default router;
