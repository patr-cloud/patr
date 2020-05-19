import { FindAccount } from 'oidc-provider';
import bcrypt from 'bcrypt';

import { getUserByUserid, getUserByUsername, createUser } from '../../../models/database-modules/user';
import { User } from '../../../models/interfaces/user';
import { saltRounds } from '../../../config/config';
import { getUserOrgs } from '../../../models/database-modules/organization';


interface BytesonusClaims {
	sub: string;
	email: string;
	userId?: string;
	organizations?: string[];
}

export default class Account {
	static findAccount: FindAccount = async (ctx, id) => {
		const account = await getUserByUsername(id);
		if (!account) {
			return undefined;
		}

		return {
			accountId: id,
			async claims(use: string, scope: string) {
				return Account.claimFromUser(account, scope);
			},
		};
	};

	static async claimFromUser(user: User, scope: string) {
		const scopes = scope.split(' ');
		// Claims to include in the User's OpenID Connect claims
		const claims: BytesonusClaims = {
			sub: user.username,
			email: user.email,
		};
		if (scopes.indexOf('bytesonus') > -1) {
			const orgs = (await getUserOrgs(user.userId)).map((g) => g.organizationId);
			claims.userId = user.userId.toString('hex');
			claims.organizations = orgs.map((b) => b.toString('hex'));
		}
		return claims;
	}

	static async authenticate(username: string, password: string) {
		const user = await getUserByUsername(username);

		if (!user) {
			return false;
		}
		if (await bcrypt.compare(password, user.password)) {
			return user.username;
		}
		return false;
	}

	static async register(username: string, email: string, password: string) {
		const passwordHash = await bcrypt.hash(password, saltRounds);
		return createUser({
			email,
			username,
			password: passwordHash,
			userId: null,
		});
	}
}
