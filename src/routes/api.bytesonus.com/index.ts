import { Router } from 'express';
import deployRouter from './deploy';
import registryRouter from './registry';
import openidRouter from './openid';
import orgRouter from './organization';

const router = Router();

router.use('/deployer', deployRouter);
router.use('/registry', registryRouter);
router.use('/openid', openidRouter);
router.use('/organization', orgRouter);

export default router;
