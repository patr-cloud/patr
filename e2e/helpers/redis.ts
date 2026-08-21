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

// Helpers for the runner connection lock: `runnerConnectionLock:{runnerId}`
// (runnerId is the non-hyphenated UUID, matching the API's id form). The value
// is the per-connection UUID held by the connected runner; debug TTL is 5s.
const runnerLockKey = (runnerId: string) => `runnerConnectionLock:${runnerId}`;

export async function runnerLockValue(runnerId: string): Promise<string | null> {
	return redis.get(runnerLockKey(runnerId));
}

// Remaining TTL in ms: -2 = no key, -1 = no expiry.
export async function runnerLockPttl(runnerId: string): Promise<number> {
	return redis.pttl(runnerLockKey(runnerId));
}

export async function deleteRunnerLock(runnerId: string): Promise<void> {
	await redis.del(runnerLockKey(runnerId));
}

// The unhashed MFA secret lives in Redis for 5 minutes during the enable flow
// (see api/src/routes/api.patr.cloud/user/mfa/get_mfa_secret.rs — keyed by
// user_id via redis::user_mfa_secret). We read it so we can compute the TOTP
// to submit through the UI.
//
// UUIDs are stored hyphenless throughout this codebase, but pg returns the
// hyphenated string form by default — strip hyphens before key construction.
export async function readMfaSetupSecret(email: string): Promise<string> {
	const user = await queryUser(email);
	if (!user) throw new Error(`No such user: ${email}`);
	const userIdHex = user.id.replace(/-/g, '');
	// The secret is written when the API handles GET /user/mfa, which the caller
	// races after opening the 2FA modal (the request is still in flight). Poll
	// briefly until it lands rather than relying on every caller to await it.
	const deadline = Date.now() + 5_000;
	for (;;) {
		const secret = await redis.get(`mfa:${userIdHex}`);
		if (secret) return secret;
		if (Date.now() >= deadline) {
			throw new Error(
				`No MFA secret in Redis for user ${email} (${user.id}); ` +
					`was GET /user/mfa called and within the 5-min TTL?`,
			);
		}
		await new Promise((r) => setTimeout(r, 100));
	}
}
