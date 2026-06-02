// Verifies that credential-change flows revoke all of the user's OTHER web
// logins (but leave the calling session alive), and that change_password
// optionally also revokes API tokens when the caller asks for it.
import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createApiTokenAPI,
  callWithApiToken,
  readMfaSetupSecret,
  computeTotp,
} from '@/prelude';
import {
  openLoginPage,
  fillLoginForm,
  submitLogin,
  waitForLoggedIn,
} from '@/helpers/ui/login';
import {
  openProfile,
  openMfaModal,
  fillMfaModalOtp,
  submitMfaModal,
  fillChangePassword,
  submitChangePassword,
} from '@/helpers/ui/profile';

test.describe('credential change revokes other sessions', () => {
  test('password change (default): other web login is kicked out, API token survives, caller stays', async ({
    browser,
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);

    const contextA = await newContext(browser);
    const pageA = await contextA.newPage();
    await openLoginPage(pageA);
    await fillLoginForm(pageA, { userId: owner.username, password: owner.password });
    await submitLogin(pageA);
    await waitForLoggedIn(pageA);

    const contextB = await newContext(browser);
    const pageB = await contextB.newPage();
    await openLoginPage(pageB);
    await fillLoginForm(pageB, { userId: owner.username, password: owner.password });
    await submitLogin(pageB);
    await waitForLoggedIn(pageB);

    try {
      const token = await createApiTokenAPI(api, owner, {
        permissions: { [owner.workspaceId]: { type: 'superAdmin' } },
      });

      // UI form omits `revokeApiTokens`, so the backend defaults it to false.
      const newPassword = 'NewPass!1Word';
      await openProfile(pageA);
      await fillChangePassword(pageA, {
        currentPassword: owner.password,
        newPassword,
      });
      await submitChangePassword(pageA);
      await expect(pageA.getByText(/Password updated successfully/i)).toBeVisible({
        timeout: 10_000,
      });

      // Default: API token keeps working.
      const tokenResp = await callWithApiToken(api, token.token, {
        clientIp: owner.clientIp,
        path: `/user`,
      });
      expect(tokenResp.status).toBe(200);

      // Other web login is dead.
      const statusB = await pageB.evaluate(async () => {
        const r = await fetch('/api/user', { credentials: 'include' });
        return r.status;
      });
      expect(statusB).toBe(401);
    } finally {
      await contextA.close();
      await contextB.close();
    }
  });

  test('MFA activate: other web login is kicked out, API token survives', async ({
    browser,
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);

    const contextA = await newContext(browser);
    const pageA = await contextA.newPage();
    await openLoginPage(pageA);
    await fillLoginForm(pageA, { userId: owner.username, password: owner.password });
    await submitLogin(pageA);
    await waitForLoggedIn(pageA);

    const contextB = await newContext(browser);
    const pageB = await contextB.newPage();
    await openLoginPage(pageB);
    await fillLoginForm(pageB, { userId: owner.username, password: owner.password });
    await submitLogin(pageB);
    await waitForLoggedIn(pageB);

    try {
      const token = await createApiTokenAPI(api, owner, {
        permissions: { [owner.workspaceId]: { type: 'superAdmin' } },
      });

      await openProfile(pageA);
      await openMfaModal(pageA);
      const secret = await readMfaSetupSecret(owner.username);
      await fillMfaModalOtp(pageA, computeTotp(secret));
      await submitMfaModal(pageA);
      await expect(
        pageA.getByText(/Two-Factor Authentication enabled/i),
      ).toBeVisible({ timeout: 10_000 });

      // MFA toggles never revoke tokens — the helper only touches web logins.
      const tokenResp = await callWithApiToken(api, token.token, {
        clientIp: owner.clientIp,
        path: `/user`,
      });
      expect(tokenResp.status).toBe(200);

      // Other web login is dead.
      const statusB = await pageB.evaluate(async () => {
        const r = await fetch('/api/user', { credentials: 'include' });
        return r.status;
      });
      expect(statusB).toBe(401);
    } finally {
      await contextA.close();
      await contextB.close();
    }
  });
});
