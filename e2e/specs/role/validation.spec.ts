import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createRoleAPI,
  getPermissionId,
  loginAs,
} from '@/prelude';
import {
  openCreateRolePage,
  fillRoleForm,
  addWorkspaceLevelPermission,
  submitCreateRole,
  expectToast,
} from '@/helpers/ui/role';

async function withUI(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('role > validation', () => {
  test('shows a client toast when role name is empty', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      await openCreateRolePage(page);
      await addWorkspaceLevelPermission(page, 'View Roles');
      await submitCreateRole(page);
      await expectToast(page, /Please enter a role name/i);
    });
  });

  test('shows a client toast when no permission is selected', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      await openCreateRolePage(page);
      await fillRoleForm(page, { name: 'something' });
      await submitCreateRole(page);
      await expectToast(page, /Please select at least one permission/i);
    });
  });

  test('rejects a role name shorter than 4 characters (server 400)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    await expect(
      createRoleAPI(api, user, user.workspaceId, {
        name: 'ab',
        permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects a role name with disallowed characters (server 400)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    await expect(
      createRoleAPI(api, user, user.workspaceId, {
        name: 'bad@name!',
        permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects a duplicate role name in the same workspace with 409', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const dup = `dup-${Date.now().toString(36)}`;
    await createRoleAPI(api, user, user.workspaceId, {
      name: dup,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    await expect(
      createRoleAPI(api, user, user.workspaceId, {
        name: dup,
        permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
      }),
    ).rejects.toThrow(/409/);
  });

  test('allows the same role name in two different workspaces', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const ws2 = await api.request<{ id: string }>('POST', '/workspace', {
      token: user.accessToken,
      clientIp: user.clientIp,
      body: { name: `wks2-${user.username}` },
    });
    const sharedName = `shared-${Date.now().toString(36)}`;
    const viewId1 = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const viewId2 = await getPermissionId(
      api,
      user.accessToken,
      ws2.id,
      user.clientIp,
      'viewRoles',
    );
    await createRoleAPI(api, user, user.workspaceId, {
      name: sharedName,
      permissions: { [viewId1]: { permissionType: 'exclude', resources: [] } },
    });
    await createRoleAPI(api, user, ws2.id, {
      name: sharedName,
      permissions: { [viewId2]: { permissionType: 'exclude', resources: [] } },
    });
    // Both succeeded — no error means it's allowed.
  });

  test('rejects a PATCH that empties the permissions map (server 400)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `patch-empty-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    await expect(
      api.request('PATCH', `/workspace/${user.workspaceId}/rbac/role/${role.id}`, {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: { permissions: {} },
      }),
    ).rejects.toThrow(/400/);
  });

  test('rejects an empty PATCH body (server 400)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const viewId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'viewRoles',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `patch-none-${Date.now().toString(36)}`,
      permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
    });
    await expect(
      api.request('PATCH', `/workspace/${user.workspaceId}/rbac/role/${role.id}`, {
        token: user.accessToken,
        clientIp: user.clientIp,
        body: {},
      }),
    ).rejects.toThrow(/400/);
  });
});
