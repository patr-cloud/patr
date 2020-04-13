import { Router } from 'express';

import auth from './auth';
import user from './user';
import mailingList from './mailing-list';

const router = Router();

router.use('/auth', auth);
router.use('/user', user);
router.use('/mailing-list', mailingList);

export default router;
