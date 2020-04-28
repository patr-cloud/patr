import { Router } from 'express';
import deployRouter from './deploy';
import registryRouter from './registry';

const router = Router();

router.use('/deploy', deployRouter);
router.use('/registry', registryRouter);

export default router;
