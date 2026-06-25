import { test, expect, newContext, createUserAccount, loginAs, expectUrl } from '@/prelude';

const GUARDED_URLS = [
  '/workspace/roles',
  '/workspace/roles/new',
  '/workspace/roles/00000000000000000000000000000000',
  '/workspace/members',
] as const;

test.describe('rbac > route guards', () => {
  for (const url of GUARDED_URLS) {
    test(`redirects logged-out visits to ${url} to /login`, async ({ browser }) => {
      const context = await newContext(browser);
      const page = await context.newPage();
      try {
        await page.goto(url, { waitUntil: 'domcontentloaded' });
        await expectUrl(page, /\/login/, { timeout: 10_000 });
      } finally {
        await context.close();
      }
    });
  }

  test('redirects users with zero workspaces from /workspace/* to /onboard', async ({
    browser,
    api,
  }) => {
    await using user = await createUserAccount(api);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user);
    const page = await context.newPage();
    try {
      await page.goto('/workspace/roles', { waitUntil: 'domcontentloaded' });
      await expectUrl(page, /\/onboard/, { timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});
