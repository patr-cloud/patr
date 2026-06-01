import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  createRoleAPI,
  deleteRoleAPI,
  getPermissionId,
  addMemberToWorkspace,
  loginAs,
  sql,
} from '@/prelude';
import {
  openRolesList,
  clickDeleteRole,
  confirmDeleteRoleModal,
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
    permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
  });
}

test.describe('role > delete', () => {
  test('deletes an unused role via UI and removes the DB row', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `del-${Date.now().toString(36)}`;
    const role = await makeRole(api, user, name);
    await withUI(browser, user, async (page) => {
      await openRolesList(page);
      await clickDeleteRole(page, name);
      await confirmDeleteRoleModal(page);
      await expectToast(page, /Role deleted successfully/i);
      await expect(page.getByRole('row').filter({ hasText: name })).toBeHidden({
        timeout: 10_000,
      });
    });
    const rows = await sql<{ id: string }>(`SELECT id FROM role WHERE id = $1`, [role.id]);
    expect(rows.length).toBe(0);
  });

  test('rejects deleting an in-use role via UI and keeps the DB row', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    const name = `in-use-${Date.now().toString(36)}`;
    const role = await makeRole(api, owner, name);
    await using member = await createUserAccount(api);
    await addMemberToWorkspace(api, owner, owner.workspaceId, member, [role.id]);
    await withUI(browser, owner, async (page) => {
      await openRolesList(page);
      await clickDeleteRole(page, name);
      await confirmDeleteRoleModal(page);
      await expectToast(page, /(in use|cannot be deleted|Failed to delete)/i);
    });
    const rows = await sql<{ id: string }>(`SELECT id FROM role WHERE id = $1`, [role.id]);
    expect(rows.length).toBe(1);
  });

  test('removes assigned users when deleted via API with remove_users=true', async ({ api }) => {
    await using owner = await createUserWithWorkspace(api);
    const role = await makeRole(api, owner, `rm-${Date.now().toString(36)}`);
    await using member = await createUserAccount(api);
    await addMemberToWorkspace(api, owner, owner.workspaceId, member, [role.id]);
    await deleteRoleAPI(api, owner, owner.workspaceId, role.id, { removeUsers: true });
    const wuRows = await sql(`SELECT 1 FROM workspace_user WHERE role_id = $1`, [role.id]);
    expect(wuRows.length).toBe(0);
    const roleRows = await sql(`SELECT id FROM role WHERE id = $1`, [role.id]);
    expect(roleRows.length).toBe(0);
  });

  test('rejects an in-use role delete via API without remove_users with 409', async ({ api }) => {
    await using owner = await createUserWithWorkspace(api);
    const role = await makeRole(api, owner, `inUseApi-${Date.now().toString(36)}`);
    await using member = await createUserAccount(api);
    await addMemberToWorkspace(api, owner, owner.workspaceId, member, [role.id]);
    await expect(deleteRoleAPI(api, owner, owner.workspaceId, role.id)).rejects.toThrow(/409/);
  });

  test('keeps the role intact when the delete modal is dismissed', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `cancel-${Date.now().toString(36)}`;
    await makeRole(api, user, name);
    await withUI(browser, user, async (page) => {
      await openRolesList(page);
      await clickDeleteRole(page, name);
      await page.keyboard.press('Escape');
      await expect(page.getByRole('row').filter({ hasText: name })).toBeVisible({
        timeout: 5_000,
      });
    });
  });

  test('exposes the trash button with aria-label="Delete role"', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `a11y-${Date.now().toString(36)}`;
    await makeRole(api, user, name);
    await withUI(browser, user, async (page) => {
      await openRolesList(page);
      const row = page.getByRole('row').filter({ hasText: name });
      await expect(row.getByRole('button', { name: /Delete role/i })).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('shows the delete modal title with the exact role name in quotes', async ({
    browser,
    api,
  }) => {
    await using user = await createUserWithWorkspace(api);
    const name = `title-${Date.now().toString(36)}`;
    await makeRole(api, user, name);
    await withUI(browser, user, async (page) => {
      await openRolesList(page);
      await clickDeleteRole(page, name);
      await expect(page.getByText(`Delete Role "${name}"`)).toBeVisible({
        timeout: 5_000,
      });
    });
  });
});
