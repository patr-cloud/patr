import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createRunnerAPI, randomRunnerName } from '@/helpers/runner-api';
import { openRunnerList, emptyStateHeading, addRunnerLink, runnerRow } from '@/helpers/ui/runner';
import { expectUrl } from '@/helpers/ui/workspace';

// List ordering, pagination and name-filter at the API layer live in the Rust
// API suite (api/tests/api/workspace/runner.rs). Here we cover only the
// dashboard surface.

async function withList(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openRunnerList(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('runner > list [UI]', () => {
  test('empty state shows heading and an add CTA', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withList(browser, user, async (page) => {
      await expect(emptyStateHeading(page)).toBeVisible();
      await expect(addRunnerLink(page).first()).toBeVisible();
    });
  });

  test('lists a runner with its name and an Unreachable status', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRunnerName();
    await createRunnerAPI(api, user, user.workspaceId, name);
    await withList(browser, user, async (page) => {
      await expect(runnerRow(page, name)).toBeVisible();
      await expect(emptyStateHeading(page)).toBeHidden();
      // A never-connected runner shows the unreachable status in its row. Scope
      // to the table; the mobile card grid (md:hidden) renders the status too
      // and its element is first in the DOM but hidden at 1280.
      await expect(
        page
          .getByRole('table')
          .getByText(/unreachable/i)
          .first(),
      ).toBeVisible();
    });
  });

  test('clicking a runner row navigates to its detail', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = randomRunnerName();
    const runner = await createRunnerAPI(api, user, user.workspaceId, name);
    await withList(browser, user, async (page) => {
      await runnerRow(page, name).click();
      await expectUrl(page, new RegExp(`/runners/${runner.id}`), { timeout: 10_000 });
    });
  });
});
