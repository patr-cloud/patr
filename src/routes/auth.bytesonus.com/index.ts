import { Router } from 'express';
import { Provider } from 'oidc-provider';
import oidcConfig from './oidc/config';

const oidc = new Provider(`https://auth.vicara.co`, oidcConfig);

oidc.proxy = true;

const router = Router();

router.use(oidc.callback);

export default router;
