import type { ApiClient } from '@/helpers/api';
import { randomIPv4 } from '@/helpers/ip';

export type User = {
  username: string;
  password: string;
  email: string;
  firstName: string;
  lastName: string;
  accessToken: string;
  refreshToken: string;
  clientIp: string;
};

export type UserHandle = User & AsyncDisposable;

// Debug builds use the always-passes Turnstile secret, which accepts any
// non-empty token. Send a placeholder so the request validates.
const TURNSTILE_TOKEN = 'e2e-placeholder-token';

// Debug builds generate OTPs from the range 0..=0, formatted as a 6-digit
// zero-padded string (api/src/utils/mod.rs `OTP_RANGE` + create_account.rs).
const DEBUG_OTP = '000000';

export async function createUserAccount(api: ApiClient): Promise<UserHandle> {
  const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
  const username = `e2euser${suffix}`;
  const password = 'E2eTest!1Password';
  const email = `${username}@example.com`;
  const firstName = 'E2E';
  const lastName = 'User';
  const clientIp = randomIPv4();

  await api.request('POST', '/auth/sign-up', {
    clientIp,
    body: {
      username,
      password,
      firstName,
      lastName,
      recoveryEmail: email,
      cfTurnstileToken: TURNSTILE_TOKEN,
    },
  });

  const tokens = await api.request<{ accessToken: string; refreshToken: string }>(
    'POST',
    '/auth/join',
    {
      clientIp,
      body: {
        username,
        verificationToken: DEBUG_OTP,
        cfTurnstileToken: TURNSTILE_TOKEN,
      },
    },
  );

  const handle: UserHandle = {
    username,
    password,
    email,
    firstName,
    lastName,
    clientIp,
    accessToken: tokens.accessToken,
    refreshToken: tokens.refreshToken,
    async [Symbol.asyncDispose]() {
      // Best-effort logout. The API doesn't expose user-deletion to the user
      // themselves, so the row stays in the DB until the next `just e2e-down`
      // wipes the volume. That's fine — usernames are random per test.
      try {
        await api.request('POST', '/auth/sign-out', {
          token: handle.accessToken,
          clientIp,
        });
      } catch {
        // ignore — token may already be expired, user already gone, etc.
      }
    },
  };

  return handle;
}
