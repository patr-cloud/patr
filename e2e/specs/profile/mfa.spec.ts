import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  loginAs,
  computeTotp,
  readMfaSetupSecret,
} from '@/prelude';
import { openProfile, openMfaModal, fillMfaModalOtp, submitMfaModal } from '@/helpers/ui/profile';

async function withProfile(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openProfile(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('profile > 2FA toggle label', () => {
  test('shows "Enable 2FA Settings" for a fresh user', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      await expect(page.getByRole('button', { name: /^Enable 2FA Settings$/ })).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('flips to "Disable 2FA Settings" after enabling MFA via UI', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withProfile(browser, user, async (page) => {
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/mfa') && r.request().method() === 'GET',
        { timeout: 10_000 },
      );
      await openMfaModal(page);
      await respPromise;
      // Tiny settle to let the redis write land before we read it.
      await page.waitForTimeout(200);
      const secret = await readMfaSetupSecret(user.username);
      const otp = computeTotp(secret);
      await fillMfaModalOtp(page, otp);
      await submitMfaModal(page);
      // Wait for the label to flip to Disable.
      await expect(page.getByRole('button', { name: /^Disable 2FA Settings$/ })).toBeVisible({
        timeout: 15_000,
      });
    });
  });
});
