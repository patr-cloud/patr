import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  createRoleAPI,
  getPermissionId,
  getRoleAPI,
  listRolesAPI,
  loginAs,
  expectUrl,
} from '@/prelude';
import {
  openRolesList,
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

test.describe('role > create', () => {
  test('creates a workspace-scoped role with a single permission via UI', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const roleName = `e2e-role-${Date.now().toString(36)}`;
    await withUI(browser, user, async (page) => {
      await openCreateRolePage(page);
      await fillRoleForm(page, { name: roleName, description: 'create test' });
      await addWorkspaceLevelPermission(page, 'View Roles');
      await submitCreateRole(page);
      await expectToast(page, /Role created successfully/i);
    });
    const roles = await listRolesAPI(api, user, user.workspaceId);
    const r = roles.find((r) => r.name === roleName);
    expect(r).toBeTruthy();
    const detail = await getRoleAPI(api, user, user.workspaceId, r!.id);
    expect(Object.keys(detail.permissions).length).toBeGreaterThanOrEqual(1);
  });

  test('creates a role with an include-specific scope (API)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const deployId = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::view',
    );
    // The workspace itself is a resource row, so its id satisfies the
    // resource FK without needing to spin up a real deployment.
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `inc-${Date.now().toString(36)}`,
      permissions: {
        [deployId]: { permissionType: 'include', resources: [user.workspaceId] },
      },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(detail.permissions[deployId].permissionType).toBe('include');
  });

  test('creates a role with an exclude-specific scope (API)', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const id = await getPermissionId(
      api,
      user.accessToken,
      user.workspaceId,
      user.clientIp,
      'deployment::view',
    );
    const role = await createRoleAPI(api, user, user.workspaceId, {
      name: `exc-${Date.now().toString(36)}`,
      permissions: { [id]: { permissionType: 'exclude', resources: [] } },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(detail.permissions[id]).toBeTruthy();
  });

  test('creates a workspace-level modifyRoles role via UI', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const roleName = `ws-${Date.now().toString(36)}`;
    await withUI(browser, user, async (page) => {
      await openCreateRolePage(page);
      await fillRoleForm(page, { name: roleName });
      await addWorkspaceLevelPermission(page, 'Modify Roles');
      await submitCreateRole(page);
      await expectToast(page, /Role created successfully/i);
    });
    const roles = await listRolesAPI(api, user, user.workspaceId);
    expect(roles.find((r) => r.name === roleName)).toBeTruthy();
  });

  test('creates a role with multiple permissions (API)', async ({ api }) => {
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
      name: `multi-${Date.now().toString(36)}`,
      permissions: {
        [viewId]: { permissionType: 'exclude', resources: [] },
        [editId]: { permissionType: 'exclude', resources: [] },
      },
    });
    const detail = await getRoleAPI(api, user, user.workspaceId, role.id);
    expect(Object.keys(detail.permissions).length).toBe(2);
  });

  test('routes to the new-role page from the header link', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      await openRolesList(page);
      await page.getByRole('link', { name: /^Create New Role$/ }).click();
      await expectUrl(page, /\/workspace\/roles\/new$/, { timeout: 10_000 });
    });
  });
});

test.describe('role > description > inline validation', () => {
  test('rejects HTML in description with inline error', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      await openCreateRolePage(page);
      await fillRoleForm(page, {
        name: 'role-' + Date.now().toString(36),
        description: '<script>x</script>',
      });
      let fired = false;
      page.on('request', (req) => {
        if (req.url().includes('/rbac/role') && req.method() === 'POST') {
          fired = true;
        }
      });
      await submitCreateRole(page);
      await expect(
        page.getByText(/Description cannot contain <, >, &, or control characters/),
      ).toBeVisible({ timeout: 5_000 });
      await page.waitForTimeout(500);
      expect(fired).toBe(false);
    });
  });
});
