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

export type PendingSignup = {
  username: string;
  password: string;
  email: string;
  firstName: string;
  lastName: string;
  clientIp: string;
};

// Calls /auth/sign-up only — no /auth/join. Used by tests that drive the
// confirm-signup flow through the UI and need a pending user_to_sign_up row.
// Retries on 429 with a fresh IP — the per-IP rate-limit bucket can be stale
// from a prior test that hit it; a fresh IP gives us a fresh bucket.
export async function createPendingSignup(
  api: ApiClient,
  overrides: Partial<PendingSignup> = {},
): Promise<PendingSignup> {
  const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
  const username = overrides.username ?? `e2euser${suffix}`;
  const password = overrides.password ?? 'E2eTest!1Password';
  const email = overrides.email ?? `${username}@example.com`;
  const firstName = overrides.firstName ?? 'E2E';
  const lastName = overrides.lastName ?? 'User';
  const explicitIp = overrides.clientIp;

  let lastErr: unknown;
  for (let attempt = 1; attempt <= 4; attempt++) {
    const clientIp = explicitIp ?? randomIPv4();
    try {
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
      return { username, password, email, firstName, lastName, clientIp };
    } catch (err) {
      lastErr = err;
      // Only retry on 429. If caller pinned an IP, don't change it.
      if (explicitIp || !String(err).includes('429')) throw err;
      await new Promise((r) => setTimeout(r, 150 * attempt));
    }
  }
  throw lastErr ?? new Error('createPendingSignup: exhausted retries');
}

// Creates a verified user AND a workspace, so the frontend's _workspaced
// route group resolves cleanly to /profile, dashboard, etc. Use this when
// the test needs to navigate to any logged-in page beyond /onboard.
export async function createUserWithWorkspace(
  api: ApiClient,
): Promise<UserHandle & { workspaceId: string }> {
  const user = await createUserAccount(api);
  const resp = await api.request<{ id: string }>('POST', '/workspace', {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: { name: `wks-${user.username}` },
  });
  return Object.assign(user, { workspaceId: resp.id });
}

export async function createUserAccount(api: ApiClient): Promise<UserHandle> {
  const { username, password, email, firstName, lastName, clientIp } =
    await createPendingSignup(api);

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
