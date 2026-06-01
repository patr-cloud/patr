import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  readMfaSetupSecret,
  computeTotp,
} from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import {
  openProfile,
  openMfaModal,
  fillMfaModalOtp,
  submitMfaModal,
  fillChangePassword,
  fillChangePasswordMfa,
  submitChangePassword,
} from '@/helpers/ui/profile';

async function loginAsNew(
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

test.describe('change-password — happy path', () => {
  test('change → new password works, old fails', async ({ browser, api }) => {
    const { context, page, user } = await loginAsNew(browser, api);
    const newPassword = 'ChangedPass!1Word';
    try {
      await openProfile(page);
      await fillChangePassword(page, {
        currentPassword: user.password,
        newPassword,
      });
      await submitChangePassword(page);
      await expect(page.getByText(/Password updated successfully/i)).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }

    // New password works.
    const ctx2 = await newContext(browser);
    const page2 = await ctx2.newPage();
    try {
      await openLoginPage(page2);
      await fillLoginForm(page2, { userId: user.username, password: newPassword });
      await submitLogin(page2);
      await waitForLoggedIn(page2);
    } finally {
      await ctx2.close();
    }

    // Old password rejected.
    const ctx3 = await newContext(browser);
    const page3 = await ctx3.newPage();
    try {
      await openLoginPage(page3);
      await fillLoginForm(page3, { userId: user.username, password: user.password });
      await submitLogin(page3);
      await expect(page3.getByText(/Incorrect password/i)).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await ctx3.close();
    }
  });
});

test.describe('change-password — client-side validation', () => {
  test('mismatched new/confirm keeps submit disabled', async ({ browser, api }) => {
    const { context, page, user } = await loginAsNew(browser, api);
    try {
      await openProfile(page);
      await fillChangePassword(page, {
        currentPassword: user.password,
        newPassword: 'NewPass!1Word',
        confirmPassword: 'DifferentPass!1',
      });
      const submit = page.getByRole('button', { name: /Update Password/ });
      await expect(submit).toBeDisabled();
    } finally {
      await context.close();
    }
  });

  test('empty current password keeps submit disabled', async ({ browser, api }) => {
    const { context, page } = await loginAsNew(browser, api);
    try {
      await openProfile(page);
      await fillChangePassword(page, {
        currentPassword: '',
        newPassword: 'NewPass!1Word',
      });
      const submit = page.getByRole('button', { name: /Update Password/ });
      await expect(submit).toBeDisabled();
    } finally {
      await context.close();
    }
  });
});

test.describe('change-password — server-side rejection', () => {
  test('wrong current password → "Current password is incorrect"', async ({ browser, api }) => {
    const { context, page } = await loginAsNew(browser, api);
    try {
      await openProfile(page);
      await fillChangePassword(page, {
        currentPassword: 'TotallyWrong!1',
        newPassword: 'NewPass!1Word',
      });
      await submitChangePassword(page);
      await expect(page.getByText(/Current password is incorrect/i)).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });

  test('new == current → server returns InvalidPassword', async ({ browser, api }) => {
    const { context, page, user } = await loginAsNew(browser, api);
    try {
      await openProfile(page);
      await fillChangePassword(page, {
        currentPassword: user.password,
        newPassword: user.password,
      });
      const respPromise = page.waitForResponse(
        (r) => r.url().includes('/user/change-password') && r.request().method() === 'POST',
      );
      await submitChangePassword(page);
      const resp = await respPromise;
      expect(resp.ok()).toBe(false);
    } finally {
      await context.close();
    }
  });
});

test.describe('change-password — MFA branch', () => {
  test('MFA-enabled user: first submit shows MFA field, second with TOTP succeeds', async ({
    browser,
    api,
  }) => {
    const { context, page, user } = await loginAsNew(browser, api);
    try {
      // Enable MFA via UI first.
      await openProfile(page);
      await openMfaModal(page);
      await page.waitForResponse(
        (r) => r.url().includes('/api/user/mfa') && r.request().method() === 'GET',
        { timeout: 10_000 },
      );
      await page.waitForTimeout(200);
      const secret = await readMfaSetupSecret(user.username);
      const enableOtp = computeTotp(secret);
      await fillMfaModalOtp(page, enableOtp);
      await submitMfaModal(page);
      await expect(page.getByText(/Two-Factor Authentication enabled/i)).toBeVisible({
        timeout: 10_000,
      });

      // Change password — first submit returns mfaRequired, MFA field appears.
      await openProfile(page); // re-render to ensure fresh component state
      await fillChangePassword(page, {
        currentPassword: user.password,
        newPassword: 'ChangedPass!1Word',
      });
      await submitChangePassword(page);
      // MFA field is OtpInput rendered in change-password section.
      await expect(page.locator('#otp-0')).toBeVisible({ timeout: 10_000 });
      const otp = computeTotp(secret);
      await fillChangePasswordMfa(page, otp);
      await submitChangePassword(page);
      await expect(page.getByText(/Password updated successfully/i)).toBeVisible({
        timeout: 15_000,
      });
    } finally {
      await context.close();
    }
  });
});
