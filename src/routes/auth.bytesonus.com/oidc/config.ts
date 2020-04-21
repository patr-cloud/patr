import { Configuration } from 'oidc-provider';
import { jwks, cookieKeys } from '../../../config/config';
import RedisAdapter from './adapter';
import Account from './account';

// TODO: Add custom interactions and basic UI for login, register, select account, forgot password, password reset, etc.
const config: Configuration = {
    adapter: RedisAdapter,
    clients: [{
        client_id: 'foo',
        redirect_uris: ['https://example.com'],
        response_types: ['id_token'],
        grant_types: ['implicit'],
        token_endpoint_auth_method: 'none',
    }],
    jwks,
    formats: {
        AccessToken: 'jwt',
    },
    features: {
        encryption: { enabled: true },
        introspection: { enabled: true },
        revocation: { enabled: true },
    },
    findAccount: Account.findAccount,
    cookies: {
        keys: cookieKeys
    }
};

export default config;
