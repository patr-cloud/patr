import {
  test,
  expect,
  newContext,
  createUserAccount,
  backdatePasswordResetToken,
  exhaustPasswordResetAttempts,
  DEBUG_OTP,
} from '@/prelude';
import {
  openForgotPassword,
  fillForgotEmail,
  submitForgot,
  expectCheckEmailView,
} from '@/helpers/ui/forgot';
import {
  openResetPassword,
  fillResetForm,
  submitReset,
} from '@/helpers/ui/reset';
import {
  openLoginPage,
  fillLoginForm,
  submitLogin,
  waitForLoggedIn,
} from '@/helpers/ui/login';

// ─────────────────────────────────────────────────────────────────────────────
// reset-password.spec.ts
//
// **THIS SPEC IS WRITTEN AHEAD OF THE UI.** No /reset-password page exists
// yet. All tests are skipped by default to avoid burning ~12 minutes timing
// out on the missing route. Once the UI lands, drop the `RESET_PASSWORD_UI=1`
// gate or just remove the `test.skip` to flip the spec on.
//
// Assumed UI contract documented in @/helpers/ui/reset.ts.
// ─────────────────────────────────────────────────────────────────────────────

const UI_READY = process.env.RESET_PASSWORD_UI === '1';
test.skip(!UI_READY, 'reset-password page is not built yet (set RESET_PASSWORD_UI=1 to run)');

async function withContext(
  browser: import('@playwright/test').Browser,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser);
  const page = await context.newPage();
  try {
    await fn(page);
  } finally {
    await context.close();
  }
}

async function requestResetFor(
  browser: import('@playwright/test').Browser,
  email: string,
) {
  await withContext(browser, async (page) => {
    await openForgotPassword(page);
    await fillForgotEmail(page, email);
    await submitForgot(page);
    await expectCheckEmailView(page);
  });
}

test.describe('reset-password [needs-ui] — happy path', () => {
  test('valid userId + OTP + new password → login with new works, old fails', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);

    const newPassword = 'NewPassw0rd!Test';
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword,
      });
      await submitReset(page);
      await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
    });

    // New password works.
    await withContext(browser, async (page) => {
      await openLoginPage(page);
      await fillLoginForm(page, { userId: user.username, password: newPassword });
      await submitLogin(page);
      await waitForLoggedIn(page);
    });

    // Old password rejected.
    await withContext(browser, async (page) => {
      await openLoginPage(page);
      await fillLoginForm(page, { userId: user.username, password: user.password });
      await submitLogin(page);
      await expect(page.getByText(/Incorrect password/i)).toBeVisible({
        timeout: 10_000,
      });
    });
  });
});

test.describe('reset-password [needs-ui] — client-side validation', () => {
  test('empty userId blocks submit', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: '',
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
      });
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/auth/reset-password')) fired = true;
      });
      await submitReset(page);
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
    });
  });

  test('OTP <6 digits keeps submit disabled', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: '12345',
        newPassword: 'NewPassw0rd!Test',
      });
      const submit = page.locator('button[type=submit]', {
        hasText: /^Reset Password$/,
      });
      await expect(submit).toBeDisabled();
    });
  });

  test('new !== confirm blocks submit', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
        confirmPassword: 'DifferentPass!1',
      });
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/auth/reset-password')) fired = true;
      });
      await submitReset(page);
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
    });
  });

  test('weak new password (no digit) blocks submit', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword: 'NoDigitsHere!',
      });
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/auth/reset-password')) fired = true;
      });
      await submitReset(page);
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
    });
  });
});

test.describe('reset-password [needs-ui] — server-side rejection', () => {
  test('wrong OTP → InvalidPasswordResetToken', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: '123456',
        newPassword: 'NewPassw0rd!Test',
      });
      const respPromise = page.waitForResponse(
        (r) =>
          r.url().includes('/auth/reset-password') &&
          r.request().method() === 'POST',
      );
      await submitReset(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('userId without a pending reset → same generic error', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    // No requestResetFor call — user has no pending reset.
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
      });
      const respPromise = page.waitForResponse(
        (r) =>
          r.url().includes('/auth/reset-password') &&
          r.request().method() === 'POST',
      );
      await submitReset(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('nonexistent userId → same generic error', async ({ browser }) => {
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: 'doesnotexist' + Date.now(),
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
      });
      const respPromise = page.waitForResponse(
        (r) =>
          r.url().includes('/auth/reset-password') &&
          r.request().method() === 'POST',
      );
      await submitReset(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('expired reset token → InvalidPasswordResetToken', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await backdatePasswordResetToken(user.username, '20 min');
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
      });
      const respPromise = page.waitForResponse(
        (r) =>
          r.url().includes('/auth/reset-password') &&
          r.request().method() === 'POST',
      );
      await submitReset(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });

  test('attempts exhausted (>5) → InvalidPasswordResetToken even on correct OTP', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    await requestResetFor(browser, user.email);
    await exhaustPasswordResetAttempts(user.username, 6);
    await withContext(browser, async (page) => {
      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword: 'NewPassw0rd!Test',
      });
      const respPromise = page.waitForResponse(
        (r) =>
          r.url().includes('/auth/reset-password') &&
          r.request().method() === 'POST',
      );
      await submitReset(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    });
  });
});

test.describe('reset-password [needs-ui] — end-to-end seam', () => {
  test('forgot → reset in one browser context', async ({ browser, api }) => {
    await using user = await createUserAccount(api);
    const newPassword = 'AnotherPass!1';
    const context = await newContext(browser);
    const page = await context.newPage();
    try {
      await openForgotPassword(page);
      await fillForgotEmail(page, user.email);
      await submitForgot(page);
      await expectCheckEmailView(page);

      await openResetPassword(page);
      await fillResetForm(page, {
        userId: user.username,
        otp: DEBUG_OTP,
        newPassword,
      });
      await submitReset(page);
      await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});
