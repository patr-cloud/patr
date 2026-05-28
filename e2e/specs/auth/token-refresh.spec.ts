import {
  test,
  expect,
  newContext,
  createUserAccount,
  backdateWebLoginExpiry,
  deleteWebLogin,
} from '@/prelude';
import {
  openLoginPage,
  fillLoginForm,
  submitLogin,
  waitForLoggedIn,
} from '@/helpers/ui/login';

// Token refresh isn't a user-driven UI flow — the SPA does it automatically.
// We log in via the UI (real browser flow), then do the refresh-token calls
// via Node `fetch` (instead of page.evaluate, which hangs in this stack on
// consecutive runs). The login is still UI-driven; only the refresh API call
// is bypassed.

async function readAuthFromBrowser(page: import('@playwright/test').Page): Promise<{
  accessToken: string;
  refreshToken: string;
  loginId: string;
}> {
  const auth = await page.evaluate(() => {
    const m = document.cookie.match(/authState=([^;]+)/);
    if (!m) throw new Error('no authState cookie');
    return JSON.parse(decodeURIComponent(m[1])) as {
      accessToken: string;
      refreshToken: string;
    };
  });
  const [loginId] = auth.refreshToken.split('.');
  return { ...auth, loginId };
}

async function refreshAccessToken(refreshToken: string): Promise<number> {
  const r = await fetch('http://localhost:3001/api/auth/access-token', {
    method: 'GET',
    headers: { Authorization: `Bearer ${refreshToken}` },
  });
  return r.status;
}

async function login(
  browser: import('@playwright/test').Browser,
  api: import('@/prelude').ApiClient,
) {
  const user = await createUserAccount(api);
  const context = await newContext(browser);
  const page = await context.newPage();
  await openLoginPage(page);
  await fillLoginForm(page, { userId: user.username, password: user.password });
  await submitLogin(page);
  await waitForLoggedIn(page);
  return { context, page, user };
}

test.describe('token-refresh', () => {
  test('refresh with valid refresh token → success', async ({ browser, api }) => {
    const { context, page } = await login(browser, api);
    try {
      const { refreshToken } = await readAuthFromBrowser(page);
      expect(await refreshAccessToken(refreshToken)).toBe(202);
    } finally {
      await context.close();
    }
  });

  test('refresh token is single-use (reuse fails)', async ({ browser, api }) => {
    const { context, page } = await login(browser, api);
    try {
      const { refreshToken } = await readAuthFromBrowser(page);
      expect(await refreshAccessToken(refreshToken)).toBe(202);
      expect(await refreshAccessToken(refreshToken)).not.toBe(202);
    } finally {
      await context.close();
    }
  });

  test('refresh with backdated web_login.token_expiry fails', async ({
    browser,
    api,
  }) => {
    const { context, page } = await login(browser, api);
    try {
      const { refreshToken, loginId } = await readAuthFromBrowser(page);
      await backdateWebLoginExpiry(loginId, '1 day');
      expect(await refreshAccessToken(refreshToken)).not.toBe(202);
    } finally {
      await context.close();
    }
  });

  test('refresh after deleting web_login row fails', async ({ browser, api }) => {
    const { context, page } = await login(browser, api);
    try {
      const { refreshToken, loginId } = await readAuthFromBrowser(page);
      await deleteWebLogin(loginId);
      expect(await refreshAccessToken(refreshToken)).not.toBe(202);
    } finally {
      await context.close();
    }
  });

  test('malformed refresh token fails', async ({ browser, api }) => {
    const { context, page } = await login(browser, api);
    try {
      expect(await refreshAccessToken('not-a-real-token')).not.toBe(202);
    } finally {
      await context.close();
    }
  });
});
