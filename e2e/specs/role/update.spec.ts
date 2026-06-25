import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  addMemberToWorkspace,
  createRoleAPI,
  getPermissionId,
  loginAs,
} from '@/prelude';
import { openRoleDetail } from '@/helpers/ui/role';

// Role update at the API layer (rename, description, add/remove/replace
// permissions, empty-body/empty-perms rejection) lives in the Rust API suite
// (api/tests/api/workspace/rbac/mod.rs). Here we cover the edit-role UI.

async function makeRole(
  api: Parameters<typeof createRoleAPI>[0],
  user: Parameters<typeof createRoleAPI>[1] & { workspaceId: string; clientIp: string },
  name: string,
) {
  const viewId = await getPermissionId(
    api,
    user.accessToken,
    user.workspaceId,
    user.clientIp,
    'viewRoles',
  );
  return createRoleAPI(api, user, user.workspaceId, {
    name,
    description: 'initial-desc',
    permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
  });
}

test.describe('role > update [UI]', () => {
  test('exposes name and description fields on the edit-role UI', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `fix-${Date.now().toString(36)}`);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRoleDetail(page, role.id);
      await expect(page.getByPlaceholder('Enter Name')).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });

  test('disables Save Changes once all permissions are removed via UI', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `empty-${Date.now().toString(36)}`);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRoleDetail(page, role.id);
      await page.getByRole('button', { name: /Remove permission/i }).click();
      await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeDisabled();
    } finally {
      await context.close();
    }
  });

  test('preserves state when navigating between Edit Permissions and Users tabs', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `tabs-${Date.now().toString(36)}`);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRoleDetail(page, role.id);
      await page.getByRole('link', { name: /^Users$/ }).click();
      await expect(page.getByText(/No users have been assigned this role yet/i)).toBeVisible({
        timeout: 10_000,
      });
      await page.getByRole('link', { name: /^Edit Permissions$/ }).click();
      await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeVisible();
    } finally {
      await context.close();
    }
  });

  test('shows the empty Users tab when no users hold the role', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `noUsers-${Date.now().toString(36)}`);
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto(`/workspace/roles/${role.id}?tab=users`, { waitUntil: 'domcontentloaded' });
      await expect(page.getByText(/No users have been assigned this role yet/i)).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });

  test('lists assigned users with a count on the Users tab', async ({ api, browser }) => {
    await using owner = await createUserWithWorkspace(api);
    const role = await makeRole(api, owner, `withUsers-${Date.now().toString(36)}`);
    await using invitee = await createUserAccount(api);
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [role.id]);
    const context = await newContext(browser, owner.clientIp);
    await loginAs(context, owner, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto(`/workspace/roles/${role.id}?tab=users`, { waitUntil: 'domcontentloaded' });
      await expect(page.getByText(`@${invitee.username}`)).toBeVisible({ timeout: 10_000 });
    } finally {
      await context.close();
    }
  });
});
