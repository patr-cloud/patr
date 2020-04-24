import { Router, RequestHandler } from 'express';
import oidc from './oidc/provider';
import authRouter from './auth';
import { errors, messages } from '../../config/errors';

const setNoCache: RequestHandler = (req, res, next) => {
    res.set('Pragma', 'no-cache');
    res.set('Cache-Control', 'no-cache, no-store');
    next();
}

const router = Router();

router.use(setNoCache);
router.use('/oauth', authRouter);

router.use('/', (req, res, next) => {
    // Fallback page for auth.bytesonus.co if accessed directly and not through a client application
    res.json({
       success: false,
       error: errors.invalidClient,
       message: messages.invalidClient 
    });
});

// router.use(oidc.callback);

export default router;
