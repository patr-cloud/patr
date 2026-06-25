import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { openMembersPage } from '@/helpers/ui/member';

test.describe('member > owner', () => {
  test('shows the workspace owner in the members list', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    const context = await newContext(browser, owner.clientIp);
    await loginAs(context, owner, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openMembersPage(page);
      // The username also appears in the user-dropdown header; scope the
      // assertion to the synthetic Owner row (pinned to top of the list).
      await expect(page.getByText(/^Owner$/).first()).toBeVisible({
        timeout: 10_000,
      });
      await expect(page.getByText(`@${owner.username}`).first()).toBeVisible();
    } finally {
      await context.close();
    }
  });

  // The creator being recorded as the workspace super-admin is covered in the
  // Rust API suite (api/tests/api/workspace/rbac/mod.rs::get_current_permissions_super_admin).

  test('hides the remove control on the owner row', async ({ browser, api }) => {
    // Frontend-only guard: backend currently allows DELETE on the owner
    // (see plan, Bug 12 — UX hides the control instead of a backend block).
    await using owner = await createUserWithWorkspace(api);
    const context = await newContext(browser, owner.clientIp);
    await loginAs(context, owner, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openMembersPage(page);
      // Owner is the only row in a fresh workspace; selecting it must not
      // surface the Remove button.
      await page.getByText(`@${owner.username}`).first().click();
      await expect(page.getByRole('button', { name: /Remove member/i })).toHaveCount(0);
    } finally {
      await context.close();
    }
  });
});
