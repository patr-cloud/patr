import { AccessTokenData } from './access-token-data';

declare global {
	namespace Express {
		interface Request {
			tokenData?: AccessTokenData;
		}
	}
}
