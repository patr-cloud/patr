import { Router, RequestHandler } from 'express';
import { errors, messages } from '../../config/errors';
import Account from './oidc/account';
import { InteractionResults } from 'oidc-provider';
import oidc from './oidc/provider';
import { getUserByUsername } from '../../models/database-modules/user';
import { sessionCheck } from './middleware';

const router = Router();

router.use((req, res, next) => {
    next();
});

router.post("/login", sessionCheck, async (req, res, next) => {
    if (!req.body.username || !req.body.password) {
        return res.status(400).json({
            success: false,
            error: errors.wrongParameters,
            message: messages.wrongParameters,
        })
    }

    const accountId = await Account.authenticate(req.body.username, req.body.password);
    if (accountId) {
        const result: InteractionResults = {
            login: {
                account: accountId,
                remember: req.body.remember === true
            },
            consent: { // Grant all scopes/claims as only trusted clients use auth.bytesonus.com
                rejectedScopes: [],
                rejectedClaims: [],
            }
        };

        const redirect = await oidc.interactionResult(req, res, result);
        return res.json({
            success: true,
            redirect,
        })
    } else {
        return res.json({
            success: false,
            error: errors.invalidCredentials,
            message: messages.invalidCredentials,
        });
    }
});

router.post("/register", sessionCheck, async (req, res, next) => {
    if (!req.body.username || !req.body.email || !req.body.password) {
        return res.status(400).json({
            success: false,
            error: errors.wrongParameters,
            message: messages.wrongParameters
        });
    }

    try {
        await Account.register(req.body.username, req.body.email, req.body.password);
        // Redirect back to interaction
        res.json({
            success: true,
            redirect: '/',
        });
    } catch (e) {
        if (e.message.startsWith('ER_DUP_ENTRY')) {
            res.json({
                success: false,
                error: errors.duplicateUser,
                message: messages.duplicateUser
            });
        }
    }
});

router.post('/select_account', async (req, res, next) => {
    // TODO: Check if user wants to continue or re-login
});


router.get('/register', sessionCheck, async (req, res, next) => {
    res.render('pages/register', {
        title: 'Register'
    });
});

router.get('/login', sessionCheck, async (req, res, next) => {
    res.render('pages/login', {
        title: "Login"
    });
});

router.get("/", async (req, res, next) => {
    let interaction;
    try {
        interaction = (await oidc.interactionDetails(req, res));
    } catch {
        // Interaction not found, user came here directly (not through a Oauth Client)
        // Redirect back to auth.bytesonus.com home page
        return res.redirect('/');
    }
    switch (interaction.prompt.name) {
        case 'select_account':
            const session = interaction.session;
            if (!session) {
                return oidc.interactionFinished(req, res, { select_account: {} }, { mergeWithLastSubmission: false });
            }

            const user = await getUserByUsername(session.accountId as string);
            res.render('select_account', {
                title: "Continue",
                email: user.email,
                username: user.username
            });
            break
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
                message: messages.serverError
            });
            break;
    }
});

router.use('/', oidc.callback);

export default router;
