import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createApiTokenAPI,
  loginAs,
  expectUrl,
} from '@/prelude';
import { openTokenList } from '@/helpers/ui/api-token';

async function withList(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openTokenList(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('api token > list', () => {
  test('shows the empty state when no tokens exist', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withList(browser, user, async (page) => {
      await expect(page.getByText(/No API Tokens Created/i)).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('lists a newly-created token by name', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    // Don't set tokenExp — backend currently rejects any date value
    // (see expiry.spec.ts FAILS-UNTIL-FIX). The Expiry column will render
    // "Never", which is what we want to verify visually anyway.
    const token = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    await withList(browser, user, async (page) => {
      await expect(page.getByText(token.name)).toBeVisible({ timeout: 10_000 });
    });
  });

  test('opens the token detail page when a row is clicked', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const token = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    await withList(browser, user, async (page) => {
      await page.getByRole('row').filter({ hasText: token.name }).click();
      await expectUrl(page, new RegExp(`/profile/api-tokens/${token.id}$`), {
        timeout: 10_000,
      });
    });
  });

  test('renders pagination when the token count exceeds the page size', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    for (let i = 0; i < 11; i++) {
      await createApiTokenAPI(api, user, {
        name: `tkn-page-${i}-${Math.random().toString(36).slice(2, 6)}`,
        permissions: { [user.workspaceId]: { type: 'superAdmin' } },
      });
    }
    await withList(browser, user, async (page) => {
      // Pagination control should be visible; assert some pagination text/nav.
      await expect(page.locator('nav, [role="navigation"]').first()).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('hides revoked tokens from the list', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const token = await createApiTokenAPI(api, user, {
      permissions: { [user.workspaceId]: { type: 'superAdmin' } },
    });
    // Revoke via API (DELETE).
    await api.request('DELETE', `/user/api-token/${token.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    await withList(browser, user, async (page) => {
      await expect(page.getByText(token.name)).toBeHidden({ timeout: 10_000 });
      await expect(page.getByText(/No API Tokens Created/i)).toBeVisible();
    });
  });
});
