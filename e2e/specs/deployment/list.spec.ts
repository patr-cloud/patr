import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { openDeploymentList, emptyStateHeading, deploymentRow } from '@/helpers/ui/deployment';

// List ordering, pagination, page-out-of-bounds (400) and filters at the API
// layer live in the Rust API suite (api/tests/api/workspace/deployment/mod.rs).
// Here we cover the dashboard list + its out-of-bounds recovery.

test.beforeAll(async () => {
  await seedMachineType();
});

test.describe('deployment > list [UI]', () => {
  test('rows render with the deployment name', async ({ browser, api }) => {
    const user = await createUserWithWorkspace(api);
    const runner = await createRunnerAPI(api, user, user.workspaceId);
    const repo = await createContainerRepo(api, user, user.workspaceId);
    const name = `ui-list-${crypto.randomUUID().slice(0, 6)}`;
    await createDeploymentAPI(api, user, user.workspaceId, {
      repositoryId: repo.id,
      runnerId: runner.id,
      name,
    });
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentList(page);
      await expect(deploymentRow(page, name)).toBeVisible({ timeout: 15_000 });
      await expect(emptyStateHeading(page)).toHaveCount(0);
    } finally {
      await context.close();
    }
  });

  // Landing on an out-of-bounds page (the API returns 400 pageOutOfBounds) steps
  // back until a valid page is found, rather than getting stuck on an error.
  test('recovers from an out-of-bounds page by stepping back', async ({ browser, api }) => {
    const user = await createUserWithWorkspace(api);
    const runner = await createRunnerAPI(api, user, user.workspaceId);
    const repo = await createContainerRepo(api, user, user.workspaceId);
    const name = `recover-${crypto.randomUUID().slice(0, 6)}`;
    await createDeploymentAPI(api, user, user.workspaceId, {
      repositoryId: repo.id,
      runnerId: runner.id,
      name,
    });
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      // count=1 with one deployment → page 3 is out of bounds; recovery walks
      // back to page 0, which shows the deployment.
      await page.goto('/deployments?page=3&count=1', { waitUntil: 'domcontentloaded' });
      await expect(deploymentRow(page, name)).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });
});
