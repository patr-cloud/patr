import { Provider, Configuration } from 'oidc-provider';
import { jwks, cookieKeys } from '../../../config/config';
import RedisAdapter from './adapter';
import Account from './account';

// TODO: Add custom interactions and basic UI for login, register, select account, forgot password, password reset, etc.
const config: Configuration = {
    adapter: RedisAdapter,
    clients: [{
        client_id: 'foo',
        client_secret: 'bar',
        redirect_uris: ['http://localhost:3000/user/oauth2/bytesonus/callback'],
        response_types: ['code'],
        grant_types: ['authorization_code'],
    }],
    jwks,
    formats: {
        AccessToken: 'jwt',
    },
    claims: {
        openid: ['sub', 'email']
    },
    interactions: {
        url(ctx, interactions) {
            return `/oauth`
        }
    },
    features: {
        devInteractions: { enabled: false },
        encryption: { enabled: true },
        introspection: { enabled: true },
        revocation: { enabled: true },
    },
    findAccount: Account.findAccount,
    cookies: {
        keys: cookieKeys
    }
};

const oidc = new Provider(`https://auth.vicara.co`, config);
oidc.proxy = true;

export default oidc;
