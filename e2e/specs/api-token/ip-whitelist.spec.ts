import { test, expect, newContext, createUserWithWorkspace, loginAs, sql } from '@/prelude';
import {
  openNewTokenPage,
  fillTokenName,
  addAllowedIp,
  enableWorkspaceCheckbox,
  selectSuperAdminRadio,
  clickCreateToken,
} from '@/helpers/ui/api-token';

// IP-whitelist enforcement at the API layer (/32 match, mismatch → 401, CIDR
// block, empty list normalization) lives in the Rust API suite
// (api/tests/api/user/api_token.rs). Here we cover the IP/CIDR chip editor UI.

async function withCreate(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openNewTokenPage(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('api token > IP/CIDR client validation', () => {
  test('commits an IP chip on Enter', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '1.2.3.4', 'Enter');
      await expect(page.getByText('1.2.3.4').first()).toBeVisible();
    });
  });

  test('commits an IP chip on Space', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '5.6.7.8', ' ');
      await expect(page.getByText('5.6.7.8').first()).toBeVisible();
    });
  });

  test('commits an IP chip on Comma', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '9.10.11.12', ',');
      await expect(page.getByText('9.10.11.12').first()).toBeVisible();
    });
  });

  test('rejects an IP with an octet over 255', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '999.1.1.1', 'Enter');
      await expect(page.getByText(/octets must be 0-255/i)).toBeVisible();
    });
  });

  test('rejects an IPv4 CIDR prefix greater than 32', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '1.2.3.4/40', 'Enter');
      await expect(page.getByText(/0-32 for IPv4/i)).toBeVisible();
    });
  });

  test('rejects an IPv6 CIDR prefix greater than 128', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreate(browser, user, async (page) => {
      await addAllowedIp(page, '::/300', 'Enter');
      await expect(page.getByText(/0-128 for IPv6/i)).toBeVisible();
    });
  });

  test('drops an IP chip from the submitted allowedIps when removed', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    let tokenId = '';
    await withCreate(browser, user, async (page) => {
      await fillTokenName(page, `chips-${Date.now().toString(36)}`);
      await addAllowedIp(page, '1.1.1.1', 'Enter');
      await addAllowedIp(page, '2.2.2.2', 'Enter');
      await page.getByRole('button', { name: 'Remove 1.1.1.1' }).click();
      await enableWorkspaceCheckbox(page, `wks-${user.username}`);
      await selectSuperAdminRadio(page);
      const respPromise = page.waitForResponse(
        (r) => r.url().endsWith('/api/user/api-token') && r.request().method() === 'POST',
        { timeout: 30_000 },
      );
      await clickCreateToken(page);
      const resp = await respPromise;
      const body = (await resp.json()) as { id: string };
      tokenId = body.id;
    });
    const rows = await sql<{ allowed_ips: string[] | null }>(
      `SELECT allowed_ips::text[] AS allowed_ips FROM user_api_token WHERE token_id = $1`,
      [tokenId],
    );
    const ips = rows[0].allowed_ips ?? [];
    expect(ips.some((s) => s.startsWith('1.1.1.1'))).toBe(false);
    expect(ips.some((s) => s.startsWith('2.2.2.2'))).toBe(true);
  });
});
