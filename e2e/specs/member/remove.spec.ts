import {
  test,
  expect,
  newContext,
  createUserAccount,
  createUserWithWorkspace,
  addMemberToWorkspace,
  getOwnUserId,
  listRolesAPI,
  loginAs,
} from '@/prelude';
import {
  openMembersPage,
  clickRemoveMember,
  confirmRemoveMember,
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

test.describe('member > remove', () => {
  test('shows confirmation text with the member full name', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row to bring up the
      // Remove control (owner row has no Remove button by design).
      await page.getByText(`@${invitee.username}`).click();
      await clickRemoveMember(page);
      const inviteeId = await getOwnUserId(api, invitee);
      const me = await api.request<{ firstName: string; lastName: string }>(
        'GET',
        `/user/${inviteeId}`,
        { token: owner.accessToken, clientIp: owner.clientIp },
      );
      const fullName = `${me.firstName} ${me.lastName}`;
      await expect(page.getByText(`Remove ${fullName} from this workspace?`)).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('keeps the member when remove is cancelled', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row to bring up the
      // Remove control (owner row has no Remove button by design).
      await page.getByText(`@${invitee.username}`).click();
      await clickRemoveMember(page);
      await page.getByRole('button', { name: /^Cancel$/ }).click();
      await expect(page.getByText(`@${invitee.username}`).first()).toBeVisible({
        timeout: 10_000,
      });
    });
  });

  test('shows a success toast after confirming remove', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await using invitee = await createUserAccount(api);
    const roles = await listRolesAPI(api, owner, owner.workspaceId);
    const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
    await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
    await withUI(browser, owner, async (page) => {
      // Owner row is auto-selected; click the invitee row to bring up the
      // Remove control (owner row has no Remove button by design).
      await page.getByText(`@${invitee.username}`).click();
      await clickRemoveMember(page);
      await confirmRemoveMember(page);
      await expectToast(page, /User removed successfully/i);
    });
  });
});
