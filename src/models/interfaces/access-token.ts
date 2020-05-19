import { JWS, JWKS } from 'jose';
import { jwks } from '../../config/config';

// TODO: Make this type strong
export default class AccessToken {
	private static key = JWKS.asKeyStore(jwks);

	public userId: string = null;

	public organizations: string[] = [];

	public sub: string; // In our case, the sub is also the username of the user

	public static parse(jwt: string) {
		const payload = JWS.verify(jwt, AccessToken.key);
		const token = new AccessToken();
		Object.assign(token, payload);
		return token;
	}
}
