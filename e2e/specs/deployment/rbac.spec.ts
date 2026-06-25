import {
  test,
  expect,
  newContext,
  loginAs,
  createUserWithWorkspace,
  createSecondMemberWithRole,
  getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, patrDeploymentBody } from '@/helpers/deployment-api';
import {
  openDeploymentList,
  createDeploymentLink,
  deploymentRow,
  emptyStateHeading,
  openDeploymentDetail,
  noPermissionsHeading,
} from '@/helpers/ui/deployment';

// Deployment RBAC at the API layer (create/view/edit/delete/start/stop gating,
// Create≠View, membership-gated empty list, cross-workspace isolation) lives in
// the Rust API suite (api/tests/api/workspace/rbac/permissions/deployment.rs).
// Here we cover dashboard control-visibility + NoPermissionsPage.

type Owner = UserHandle & { workspaceId: string };

test.beforeAll(async () => {
  await seedMachineType();
});

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
  return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

async function ownerWithDeployment(api: ApiClient) {
  const owner = await createUserWithWorkspace(api);
  const runner = await createRunnerAPI(api, owner, owner.workspaceId);
  const repo = await createContainerRepo(api, owner, owner.workspaceId);
  const dep = await createDeploymentAPI(api, owner, owner.workspaceId, {
    repositoryId: repo.id,
    runnerId: runner.id,
  });
  return { owner, runner, repo, dep };
}

test.describe('deployment > RBAC [UI]', () => {
  test('a view-only member sees deployments but no Create Deployment CTA', async ({
    browser,
    api,
  }) => {
    const { owner, dep } = await ownerWithDeployment(api);
    const viewId = await permId(api, owner, 'deployment::view');
    await using member = await createSecondMemberWithRole(api, owner, {
      [viewId]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, member.clientIp);
    await loginAs(context, member, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentList(page);
      await expect(deploymentRow(page, dep.name)).toBeVisible({ timeout: 15_000 });
      await expect(createDeploymentLink(page)).toHaveCount(0);
    } finally {
      await context.close();
    }
  });

  test('a create-only member hits NoPermissionsPage on a deployment detail', async ({
    browser,
    api,
  }) => {
    const owner = await createUserWithWorkspace(api);
    const runner = await createRunnerAPI(api, owner, owner.workspaceId);
    const repo = await createContainerRepo(api, owner, owner.workspaceId);
    const createId = await permId(api, owner, 'deployment::create');
    await using member = await createSecondMemberWithRole(api, owner, {
      [createId]: { permissionType: 'exclude', resources: [] },
    });
    // The member creates a deployment (allowed) but cannot view it.
    const created = await api.request<{ id: string }>(
      'POST',
      `/workspace/${owner.workspaceId}/deployment`,
      {
        token: member.accessToken,
        clientIp: member.clientIp,
        body: patrDeploymentBody({ repositoryId: repo.id, runnerId: runner.id }),
      },
    );
    const context = await newContext(browser, member.clientIp);
    await loginAs(context, member, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentDetail(page, created.id);
      await expect(noPermissionsHeading(page)).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });

  test('a member with no deployment permission sees the empty state', async ({ browser, api }) => {
    const { owner } = await ownerWithDeployment(api);
    const viewRoles = await permId(api, owner, 'viewRoles');
    await using member = await createSecondMemberWithRole(api, owner, {
      [viewRoles]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, member.clientIp);
    await loginAs(context, member, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentList(page);
      await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });
});
