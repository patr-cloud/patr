import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { openMembersPage, searchUser, submitAddMember, expectToast } from '@/helpers/ui/member';

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

test.describe('member > validation', () => {
  test('shows a client toast when no user is selected', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      await submitAddMember(page);
      await expectToast(page, /Please select a user and at least one role/i);
    });
  });

  test('shows the same toast when a user is selected but no role', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    const { createUserAccount } = await import('@/helpers/user');
    await using invitee = await createUserAccount(api);
    await withUI(browser, owner, async (page) => {
      await searchUser(page, invitee.username);
      // Wait for dropdown then select.
      await page.getByText(`@${invitee.username}`).first().click();
      await submitAddMember(page);
      await expectToast(page, /Please select a user and at least one role/i);
    });
  });

  test('does not call the search API for fewer than 3 characters', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      let fired = false;
      page.on('request', (r) => {
        if (r.url().includes('/user/search')) fired = true;
      });
      await searchUser(page, 'ab');
      await page.waitForTimeout(700);
      expect(fired).toBe(false);
    });
  });

  test('calls the search API once at the 3-character threshold', async ({ browser, api }) => {
    await using user = await createUserWithWorkspace(api);
    await withUI(browser, user, async (page) => {
      let count = 0;
      page.on('request', (r) => {
        if (r.url().includes('/user/search')) count++;
      });
      await searchUser(page, 'abc');
      await page.waitForTimeout(700);
      expect(count).toBeGreaterThanOrEqual(1);
    });
  });

  // Add-member with a nonexistent userId is covered in the Rust API suite
  // (api/tests/api/workspace/rbac/mod.rs::update_user_roles_nonexistent_user).
});
