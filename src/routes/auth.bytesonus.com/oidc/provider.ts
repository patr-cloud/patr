import { Provider, Configuration, interactionPolicy } from 'oidc-provider';
import { jwks, cookieKeys } from '../../../config/config';
import RedisAdapter from './adapter';
import Account from './account';


const interactions = interactionPolicy.base();

const selectAccount = new interactionPolicy.Prompt({
	name: 'select_account',
});

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
		openid: ['sub', 'email'],
	},
	interactions: {
		policy: interactions,
		url(ctx, _interactions) {
			return '/oauth';
		},
	},
	features: {
		devInteractions: { enabled: false },
		encryption: { enabled: true },
		introspection: { enabled: true },
		revocation: { enabled: true },
	},
	findAccount: Account.findAccount,
	cookies: {
		keys: cookieKeys,
	},
};

const oidc = new Provider('https://auth.vicara.co', config);
oidc.proxy = true;

selectAccount.checks.push(new interactionPolicy.Check('User session exists, select account', 'Select user account', async (ctx) => {
	if (ctx.oidc.session.accountId()) {
		// If there is a session, make sure we haven't already done a login prompt
		if (ctx.oidc.result && (ctx.oidc.result.login || ctx.oidc.result.select_account)) {
			return false;
		}
	} else {
		return false;
	}
	return true;
}));

interactions.add(selectAccount, 0);

export default oidc;
