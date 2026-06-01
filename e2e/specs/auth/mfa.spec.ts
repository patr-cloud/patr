import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  readMfaSetupSecret,
  computeTotp,
} from '@/prelude';
import {
  openLoginPage,
  fillLoginForm,
  fillMfaOtp,
  submitLogin,
  waitForLoggedIn,
} from '@/helpers/ui/login';
import {
  openProfile,
  openMfaModal,
  fillMfaModalOtp,
  submitMfaModal,
  signOut,
} from '@/helpers/ui/profile';

async function login(
  browser: import('@playwright/test').Browser,
  api: import('@/prelude').ApiClient,
) {
  const user = await createUserWithWorkspace(api);
  const context = await newContext(browser);
  const page = await context.newPage();
  await openLoginPage(page);
  await fillLoginForm(page, { userId: user.username, password: user.password });
  await submitLogin(page);
  await waitForLoggedIn(page);
  return { context, page, user };
}

// Drive the enable flow through the UI, capturing the secret so subsequent
// steps (login, disable) can compute matching TOTPs.
async function enableMfaViaUi(
  page: import('@playwright/test').Page,
  username: string,
): Promise<string> {
  await openProfile(page);
  await openMfaModal(page);
  // Wait for the QR + Redis secret to land. The modal queries GET /user/mfa
  // which writes the secret to Redis.
  await page.waitForResponse(
    (r) => r.url().includes('/api/user/mfa') && r.request().method() === 'GET',
    { timeout: 10_000 },
  );
  // Tiny settle to let the Redis write land before we read it.
  await page.waitForTimeout(200);
  const secret = await readMfaSetupSecret(username);
  const otp = computeTotp(secret);
  await fillMfaModalOtp(page, otp);
  await submitMfaModal(page);
  // Modal closes — assert toast or absence of modal.
  await expect(page.getByText(/Two-Factor Authentication enabled/i)).toBeVisible({
    timeout: 10_000,
  });
  return secret;
}

test.describe('mfa — enable flow', () => {
  test('correct TOTP enables MFA', async ({ browser, api }) => {
    const { context, page, user } = await login(browser, api);
    try {
      await enableMfaViaUi(page, user.username);
    } finally {
      await context.close();
    }
  });

  test('wrong TOTP shows error, modal stays open', async ({ browser, api }) => {
    const { context, page } = await login(browser, api);
    try {
      await openProfile(page);
      await openMfaModal(page);
      await page.waitForResponse(
        (r) => r.url().includes('/api/user/mfa') && r.request().method() === 'GET',
        { timeout: 10_000 },
      );
      await fillMfaModalOtp(page, '000000'); // random/wrong
      await submitMfaModal(page);
      await expect(page.getByText(/Failed to verify OTP/i)).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});

test.describe('mfa — disable flow', () => {
  test('correct TOTP disables MFA', async ({ browser, api }) => {
    const { context, page, user } = await login(browser, api);
    try {
      const secret = await enableMfaViaUi(page, user.username);
      // Reopen modal for disable. The button now says "Disable 2FA Settings".
      await openMfaModal(page);
      // Disable flow doesn't fetch a new secret (no GET /user/mfa); we reuse
      // the captured secret to compute a current TOTP.
      const otp = computeTotp(secret);
      await fillMfaModalOtp(page, otp);
      await submitMfaModal(page);
      // The toast fires + auto-dismisses in 5s; race-prone to assert on it.
      // Instead, assert the button text flips back to "Enable 2FA Settings"
      // — that's the durable post-disable signal.
      await expect(page.getByRole('button', { name: /Enable 2FA Settings/ })).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });

  test('wrong TOTP fails to disable', async ({ browser, api }) => {
    const { context, page, user } = await login(browser, api);
    try {
      await enableMfaViaUi(page, user.username);
      await openMfaModal(page);
      await fillMfaModalOtp(page, '111111');
      await submitMfaModal(page);
      await expect(page.getByText(/Failed to verify OTP/i)).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});

test.describe('mfa — login with MFA', () => {
  test('user with MFA: first submit reveals MFA field, second succeeds', async ({
    browser,
    api,
  }) => {
    const { context, page, user } = await login(browser, api);
    let secret: string;
    try {
      secret = await enableMfaViaUi(page, user.username);
      await signOut(page);
    } finally {
      await context.close();
    }

    // Fresh context, attempt login again.
    const context2 = await newContext(browser);
    const page2 = await context2.newPage();
    try {
      await openLoginPage(page2);
      await fillLoginForm(page2, { userId: user.username, password: user.password });
      await submitLogin(page2);
      // First submit reveals MFA prompt (server returns mfaRequired).
      await expect(page2.locator('#otp-0')).toBeVisible({ timeout: 10_000 });
      const otp = computeTotp(secret);
      await fillMfaOtp(page2, otp);
      // Refresh Turnstile state and re-submit.
      await submitLogin(page2);
      await waitForLoggedIn(page2);
    } finally {
      await context2.close();
    }
  });

  test('user with MFA: wrong TOTP keeps user on /login', async ({ browser, api }) => {
    const { context, page, user } = await login(browser, api);
    try {
      await enableMfaViaUi(page, user.username);
      await signOut(page);
    } finally {
      await context.close();
    }

    const context2 = await newContext(browser);
    const page2 = await context2.newPage();
    try {
      await openLoginPage(page2);
      await fillLoginForm(page2, { userId: user.username, password: user.password });
      await submitLogin(page2);
      await expect(page2.locator('#otp-0')).toBeVisible({ timeout: 10_000 });
      await fillMfaOtp(page2, '000000');
      const respPromise = page2.waitForResponse(
        (r) => r.url().includes('/auth/sign-in') && r.request().method() === 'POST',
      );
      await submitLogin(page2);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
      await expect(page2).toHaveURL(/\/login$/);
    } finally {
      await context2.close();
    }
  });
});
