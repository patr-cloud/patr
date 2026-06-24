import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createContainerRepo, randomRepoName } from '@/helpers/registry';
import {
  openRegistryList,
  emptyStateHeading,
  createRepoLink,
  repoRow,
} from '@/helpers/ui/container-registry';
import { expectUrl } from '@/helpers/ui/workspace';

// List ordering, pagination, page-out-of-bounds and filtering at the API layer
// live in the Rust API suite (api/tests/api/workspace/container_registry.rs).
// Here we cover only the dashboard surface.

async function withList(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openRegistryList(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('container registry > list [UI]', () => {
  test('empty state shows heading and a create CTA, no header button', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withList(browser, user, async (page) => {
      await expect(emptyStateHeading(page)).toBeVisible();
      // The create CTA in the empty state is a link to /container-registry/new.
      await expect(createRepoLink(page).first()).toBeVisible();
    });
  });

  test('lists repositories and shows the header create button', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRepoName();
    await createContainerRepo(api, user, user.workspaceId, name);
    await withList(browser, user, async (page) => {
      await expect(repoRow(page, name)).toBeVisible();
      await expect(emptyStateHeading(page)).toBeHidden();
      await expect(createRepoLink(page).first()).toBeVisible();
    });
  });

  test('clicking a row navigates to the repository detail', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRepoName();
    const repo = await createContainerRepo(api, user, user.workspaceId, name);
    await withList(browser, user, async (page) => {
      await repoRow(page, name).click();
      await expectUrl(page, new RegExp(`/container-registry/${repo.id}`), { timeout: 10_000 });
    });
  });
});
