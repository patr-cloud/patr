import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';
import {
  openRegistryCreate,
  fillRepoName,
  submitCreateRepo,
  registryPathPreview,
  nameErrorAlert,
  openRegistryList,
  repoRow,
} from '@/helpers/ui/container-registry';
import { createContainerRepo, randomRepoName } from '@/helpers/registry';

// Repo creation is driven through the dashboard. The create form blocks only
// empty/whitespace client-side; everything else is POSTed and any server
// rejection (too short, bad charset, uppercase, duplicate) collapses into one
// generic "Failed to create repository" alert — the UI cannot distinguish the
// 400/409/500 underneath. That precision, plus reusable-after-delete and
// cross-workspace uniqueness, lives in the Rust API suite
// (api/tests/api/workspace/container_registry.rs).

async function withCreatePage(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openRegistryCreate(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

function trackCreatePosts(page: import('@playwright/test').Page): () => number {
  let count = 0;
  page.on('request', (req) => {
    if (req.method() === 'POST' && /\/api\/workspace\/[^/]+\/container-registry$/.test(req.url())) {
      count += 1;
    }
  });
  return () => count;
}

test.describe('container registry > create [UI]', () => {
  test('creates a repo: success toast + navigate to detail', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRepoName();
    await withCreatePage(browser, user, async (page) => {
      await fillRepoName(page, name);
      await submitCreateRepo(page);
      await expectToast(page, /Repository created successfully/i);
      await expectUrl(page, /\/container-registry\/[0-9a-f]+/, { timeout: 10_000 });
    });
  });

  test('accepts a dot/underscore-separated lowercase name', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `aa.bb_cc-${crypto.randomUUID().replace(/-/g, '').slice(0, 6)}`;
    await withCreatePage(browser, user, async (page) => {
      await fillRepoName(page, name);
      await submitCreateRepo(page);
      await expectToast(page, /Repository created successfully/i);
      await expectUrl(page, /\/container-registry\/[0-9a-f]+/, { timeout: 10_000 });
    });
  });

  test('trims surrounding whitespace from the stored name', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRepoName();
    await withCreatePage(browser, user, async (page) => {
      await fillRepoName(page, `  ${name}  `);
      await submitCreateRepo(page);
      await expectUrl(page, /\/container-registry\/[0-9a-f]+/, { timeout: 10_000 });
    });
    // The list row shows the trimmed name.
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRegistryList(page);
      await expect(repoRow(page, name)).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('shows the live registry-path preview once a name is typed', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreatePage(browser, user, async (page) => {
      await expect(registryPathPreview(page)).toBeHidden();
      await fillRepoName(page, 'mypreview');
      const preview = registryPathPreview(page);
      await expect(preview).toBeVisible();
      await expect(preview).toContainText(`${user.workspaceId}/mypreview`);
    });
  });

  test('empty name: inline error and no network call', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreatePage(browser, user, async (page) => {
      const posts = trackCreatePosts(page);
      await submitCreateRepo(page);
      await expect(nameErrorAlert(page)).toBeVisible();
      await page.waitForTimeout(500);
      expect(posts()).toBe(0);
    });
  });

  test('whitespace-only name: blocked client-side, no network call', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreatePage(browser, user, async (page) => {
      const posts = trackCreatePosts(page);
      await fillRepoName(page, '   ');
      await submitCreateRepo(page);
      await expect(nameErrorAlert(page)).toBeVisible();
      await page.waitForTimeout(500);
      expect(posts()).toBe(0);
    });
  });

  test('a server-rejected name (uppercase) surfaces a generic create error', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    await withCreatePage(browser, user, async (page) => {
      // Uppercase passes the empty/whitespace client guard, POSTs, and 400s.
      await fillRepoName(page, 'AbCd');
      await submitCreateRepo(page);
      await expectToast(page, /Failed to create repository/i);
      await expectUrl(page, /\/container-registry\/new/, { timeout: 5_000 });
    });
  });

  test('duplicate name surfaces a generic create error', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRepoName();
    await createContainerRepo(api, user, user.workspaceId, name);
    await withCreatePage(browser, user, async (page) => {
      await fillRepoName(page, name);
      await submitCreateRepo(page);
      await expectToast(page, /Failed to create repository/i);
      await expectUrl(page, /\/container-registry\/new/, { timeout: 5_000 });
    });
  });
});
