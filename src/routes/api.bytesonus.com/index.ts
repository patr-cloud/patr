import { Router } from 'express';
import deployRouter from './deploy';
import registryRouter from './registry';
import openidRouter from './openid';
import groupRouter from './group';

const router = Router();

router.use('/deployer', deployRouter);
router.use('/registry', registryRouter);
router.use('/openid', openidRouter);
router.use('/group', groupRouter);

export default router;
