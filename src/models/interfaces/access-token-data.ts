import { User } from './user';
import { encode, decode } from 'jwt-handler';
import { jwtSecret } from '../../config/config';

export class AccessTokenData {

	public static TOKEN_VALIDITY = 1000 * 60 * 60 * 24 * 3; // 3 days

	public iss = 'https://api.bytesonus.com';
	public aud = 'https://*.bytesonus.com';
	public iat: number;
	public typ = 'accessToken';
	public exp: number;
	public user: User = null;
	public groups: string[] = [];

	public static async parse(data: string): Promise<AccessTokenData> {
		const result = await decode(jwtSecret, data);
		const tokenData = new AccessTokenData();
		Object.assign(tokenData, result.payload);
		return tokenData;
	}

	constructor(iat?: number) {
		if (iat === undefined) {
			iat = Date.now();
		}
		this.iat = iat;
		this.exp = iat + AccessTokenData.TOKEN_VALIDITY;
	}

	public async toJwtString(): Promise<string> {
		return encode(jwtSecret, this);
	}
}
