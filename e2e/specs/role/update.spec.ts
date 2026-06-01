import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  addMemberToWorkspace,
  createRoleAPI,
  getPermissionId,
  getRoleAPI,
  updateRoleAPI,
  loginAs,
} from '@/prelude';
import { openRoleDetail } from '@/helpers/ui/role';

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

test.describe('role > update', () => {
  test('renames a role via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `n-${Date.now().toString(36)}`);
    const newName = `renamed-${Date.now().toString(36)}`;
    await updateRoleAPI(api, user, user.workspaceId, role.id, { name: newName });
    const got = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(got.name).toBe(newName);
  });

  test('updates the description via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `d-${Date.now().toString(36)}`);
    await updateRoleAPI(api, user, user.workspaceId, role.id, { description: 'new-desc' });
    const got = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(got.description).toBe('new-desc');
  });

  test('updates name + description + permissions in a single call', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const role = await makeRole(api, user, `all-${Date.now().toString(36)}`);
    const newName = `bigchange-${Date.now().toString(36)}`;
    const wsViewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'modifyRoles',
    );
    await updateRoleAPI(api, user, user.workspaceId, role.id, {
      name: newName,
      description: 'all-updated',
      permissions: { [wsViewId]: { permissionType: 'exclude', resources: [] } },
    });
    const got = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(got.name).toBe(newName);
    expect(got.description).toBe('all-updated');
    expect(got.permissions[wsViewId]).toBeTruthy();
  });

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

  test('adds a permission via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::view',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `add-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    const editId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::edit',
    );
    await api.request('PATCH', `/workspace/${user.workspaceId}/rbac/role/${role.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
      body: {
        permissions: {
          [viewId]: { permissionType: 'exclude', resources: [] },
          [editId]: { permissionType: 'exclude', resources: [] },
        },
      },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(Object.keys(detail.permissions).length).toBe(2);
  });

  test('removes a permission via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::view',
    );
    const editId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::edit',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `rm-${Date.now().toString(36)}`,
      permissions: {
        [viewId]: { permissionType: 'exclude', resources: [] },
        [editId]: { permissionType: 'exclude', resources: [] },
      },
    });
    await api.request('PATCH', `/workspace/${user.workspaceId}/rbac/role/${role.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
      body: { permissions: { [viewId]: { permissionType: 'exclude', resources: [] } } },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(detail.permissions[editId]).toBeUndefined();
  });

  test('replaces the full permission set via API', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::view',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `rep-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    const wsViewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    await api.request('PATCH', `/workspace/${user.workspaceId}/rbac/role/${role.id}`, {
      token: user.accessToken,
      clientIp: user.clientIp,
      body: { permissions: { [wsViewId]: { permissionType: 'exclude', resources: [] } } },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(detail.permissions[viewId]).toBeUndefined();
    expect(detail.permissions[wsViewId]).toBeTruthy();
  });

  test('disables Save Changes once all permissions are removed via UI', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `empty-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
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
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `tabs-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
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
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `noUsers-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
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
    const viewId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, owner, owner.workspaceId, {
      name: `withUsers-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    await using invitee = await createUserAccount(api);
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [role.id]);
    const context = await newContext(browser, owner.clientIp);
    await loginAs(context, owner, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await page.goto(`/workspace/roles/${role.id}?tab=users`, { waitUntil: 'domcontentloaded' });
      await expect(page.getByText(`@${invitee.username}`)).toBeVisible({
        timeout: 10_000,
      });
    } finally {
      await context.close();
    }
  });
});
