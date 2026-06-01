import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  addMemberToWorkspace,
  listRolesAPI,
  setUserRolesAPI,
  getOwnUserId,
  loginAs,
  sql,
} from '@/prelude';
import {
  openMembersPage,
  clickEditRoles,
  removeRoleChip,
  addRoleViaChipDropdown,
  saveMemberRoles,
  cancelMemberRolesEdit,
  expectToast,
} from '@/helpers/ui/member';

async function withUI(
  browser: import('@playwright/test').Browser,
  user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
  fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
  const context = await newContext(browser, user.clientIp);
  await loginAs(context, user, { workspaceId: user.workspaceId });
  const page = await context.newPage();
  try {
    await openMembersPage(page);
    await fn(page);
  } finally {
    await context.close();
  }
}

test.describe('member > roles', () => {
  test('adds a role via the chip dropdown and persists it', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    const r2 = roles.find((r) => /Deployment: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row first.
      await page.getByText(`@${invitee.username}`).click();
      await clickEditRoles(page);
      await addRoleViaChipDropdown(page, r2.name);
      await saveMemberRoles(page);
      await expectToast(page, /Roles updated successfully/i);
    });
    const inviteeId = await getOwnUserId(api, invitee);
    const rows = await sql<{ role_id: string }>(
      `SELECT role_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2`,
      [owner.workspaceId, inviteeId],
    );
    expect(rows.length).toBe(2);
  });

  test('removes a role chip and persists the deletion', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    const r2 = roles.find((r) => /Deployment: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id, r2.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row first.
      await page.getByText(`@${invitee.username}`).click();
      await clickEditRoles(page);
      await removeRoleChip(page, r2.name);
      await saveMemberRoles(page);
      await expectToast(page, /Roles updated successfully/i);
    });
    const inviteeId = await getOwnUserId(api, invitee);
    const rows = await sql<{ role_id: string }>(
      `SELECT role_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2`,
      [owner.workspaceId, inviteeId],
    );
    expect(rows.length).toBe(1);
  });

  test('discards local edits when Cancel is clicked', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row first.
      await page.getByText(`@${invitee.username}`).click();
      await clickEditRoles(page);
      await removeRoleChip(page, r1.name);
      await cancelMemberRolesEdit(page);
    });
    const inviteeId = await getOwnUserId(api, invitee);
    const rows = await sql(
      `SELECT 1 FROM workspace_user WHERE workspace_id = $1 AND user_id = $2`,
      [owner.workspaceId, inviteeId],
    );
    expect(rows.length).toBe(1);
  });

  test('links the "create a new role" hint to /workspace/roles/new', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row first.
      await page.getByText(`@${invitee.username}`).click();
      await clickEditRoles(page);
      const link = page.getByRole('link', { name: /create a new role/i });
      await expect(link).toHaveAttribute('href', '/workspace/roles/new');
    });
  });

  test('removes the member from the workspace when their roles are emptied', async ({ api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    const inviteeId = await getOwnUserId(api, invitee);
    await setUserRolesAPI(api, owner, owner.workspaceId, inviteeId, []);
    const rows = await sql(
      `SELECT 1 FROM workspace_user WHERE workspace_id = $1 AND user_id = $2`,
      [owner.workspaceId, inviteeId],
    );
    expect(rows.length).toBe(0);
  });

  test('rejects POST user roles with a non-existent roleId (4xx)', async ({ api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const inviteeId = await getOwnUserId(api, invitee);
    await expect(
      setUserRolesAPI(api, owner, owner.workspaceId, inviteeId, [
        crypto.randomUUID().replace(/-/g, ''),
      ]),
    ).rejects.toThrow(/4\d\d/);
  });

  test('rejects POST user roles with a role from a different workspace', async ({ api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using otherOwner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const otherRoles = await listRolesAPI(api, otherOwner, otherOwner.workspaceId);
    const otherRoleId = otherRoles[0].id;
    const inviteeId = await getOwnUserId(api, invitee);
    await expect(
      setUserRolesAPI(api, owner, owner.workspaceId, inviteeId, [otherRoleId]),
    ).rejects.toThrow(/4\d\d/);
  });
});
