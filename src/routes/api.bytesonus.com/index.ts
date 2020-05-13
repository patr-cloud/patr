import { Router } from 'express';
import deployRouter from './deploy';
import registryRouter from './registry';
import openidRouter from './openid';

const router = Router();

router.use('/deploy', deployRouter);
router.use('/registry', registryRouter);
router.use('/openid', openidRouter);

export default router;
