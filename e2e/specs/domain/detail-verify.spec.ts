import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { markDomainVerified } from '@/helpers/db';
import { addDomainAPI, getDomainInfoAPI } from '@/helpers/domain-api';
import { openDomainDetail, verifyButton } from '@/helpers/ui/domain';
import { expectToast } from '@/helpers/ui/workspace';

// Verify/get-info behavior at the API layer (returns-false-offline, never
// demotes, anti-enum 401, deleted→401) lives in the Rust API suite
// (api/tests/api/workspace/domain.rs). Here we cover only the dashboard surface.

async function withDetail(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  id: string,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openDomainDetail(page, id);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('domain > detail [UI]', () => {
  // Bug: the verify handler shows a success toast even though verification
  // failed (the chip stays "not verified").
  test('clicking Verify shows a misleading success toast but stays unverified', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const added = await addDomainAPI(api, user, user.workspaceId);
    await withDetail(browser, user, added.id, async (page) => {
      await expect(verifyButton(page)).toBeVisible();
      await verifyButton(page).click();
      await expectToast(page, /Domain verification initiated/i);
      // It does not become verified.
      const info = await getDomainInfoAPI(api, user, user.workspaceId, added.id);
      expect(info.isVerified).toBe(false);
    });
  });

  test('a verified domain shows no Verify button', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const added = await addDomainAPI(api, user, user.workspaceId);
    await markDomainVerified(added.id);
    await withDetail(browser, user, added.id, async (page) => {
      // Give the detail query a moment to load the verified state.
      await page.waitForTimeout(1000);
      await expect(verifyButton(page)).toHaveCount(0);
    });
  });
});
