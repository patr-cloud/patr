import jwt from 'jsonwebtoken';
import { v1 as uuidv1 } from 'uuid';
import { JWT_SECRET } from '@/helpers/config';

/**
 * Decode an existing access-token JWT, override its time-related claims with
 * past timestamps, and re-sign with the same secret. Used by the refresh-flow
 * e2e to simulate "access expired, refresh still fresh" without mutating any
 * server state.
 *
 * We spread the existing claims verbatim so that any new fields added to
 * `AccessTokenData` on the backend don't require this helper to change — only
 * `exp`, `nbf`, `iat`, and `jti` are overridden. The JTI must be a UUIDv1
 * (with embedded timestamp) because the backend's web_dashboard middleware
 * extracts a timestamp from it for an age check; a v4 (random) UUID has no
 * timestamp and surfaces as MalformedAccessToken, which the SPA's refresh
 * path doesn't recognise.
 */
export function expireAccessTokenJwt(currentAccessToken: string): string {
  const decoded = jwt.decode(currentAccessToken);
  if (typeof decoded !== 'object' || decoded === null) {
    throw new Error('Could not decode access token JWT');
  }
  const now = Math.floor(Date.now() / 1000);
  return jwt.sign(
    {
      ...decoded,
      exp: now - 60,
      nbf: now - 120,
      iat: now - 120,
      jti: uuidv1().replace(/-/g, ''),
    },
    JWT_SECRET,
    { algorithm: 'HS256' },
  );
}
