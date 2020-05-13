import { JWS, JWKS } from 'jose';
import { jwks } from '../../config/config';

// TODO: Make this type strong
export default class AccessToken {
	private static key = JWKS.asKeyStore(jwks);

	public userId: string = null;

	public groups: string[] = [];

	public static parse(jwt: string) {
		const payload = JWS.verify(jwt, AccessToken.key);
		const token = new AccessToken();
		Object.assign(token, payload);
		return token;
	}
}
