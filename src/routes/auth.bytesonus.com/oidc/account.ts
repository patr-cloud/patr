import { FindAccount } from 'oidc-provider';
import bcrypt from 'bcrypt';

import { getUserByUserid, getUserByUsername, createUser } from '../../../models/database-modules/user';
import { User } from '../../../models/interfaces/user';
import { saltRounds } from '../../../config/config';

export default class Account {
	static findAccount: FindAccount = async (ctx, id) => {
		const account = await getUserByUsername(id);
		if (!account) {
			return undefined;
		}

		return {
			accountId: id,
			async claims() {
				return Account.claimFromUser(account);
			},
		};
	};

	static claimFromUser(user: User) {
		// Claims to include in the User's OpenID Connect claims
		return {
			sub: user.username,
			email: user.email,
		};
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
		return createUser(email, username, passwordHash);
	}
}
