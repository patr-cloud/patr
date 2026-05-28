import { Redis } from 'ioredis';
import { queryUser } from '@/helpers/db';

// Single shared connection. Redis is on the host port mapped by docker-compose.
const redis = new Redis({
  host: '127.0.0.1',
  port: 16379,
  lazyConnect: true,
});

export async function disposeRedis(): Promise<void> {
  redis.disconnect();
}

// The unhashed MFA secret lives in Redis for 5 minutes during the enable flow
// (see api/src/routes/api.patr.cloud/user/mfa/get_mfa_secret.rs — keyed by
// user_id via redis::user_mfa_secret). We read it so we can compute the TOTP
// to submit through the UI.
//
// UUIDs are stored hyphenless throughout this codebase, but pg returns the
// hyphenated string form by default — strip hyphens before key construction.
export async function readMfaSetupSecret(username: string): Promise<string> {
  const user = await queryUser(username);
  if (!user) throw new Error(`No such user: ${username}`);
  const userIdHex = user.id.replace(/-/g, '');
  const secret = await redis.get(`mfa:${userIdHex}`);
  if (!secret) {
    throw new Error(
      `No MFA secret in Redis for user ${username} (${user.id}); ` +
      `was GET /user/mfa called and within the 5-min TTL?`,
    );
  }
  return secret;
}
