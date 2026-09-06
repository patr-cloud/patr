import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	addMemberToWorkspace,
	listRolesAPI,
	loginAs,
	expectUrl,
} from '@/prelude';
import { openMembersPage } from '@/helpers/ui/member';

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

test.describe('member > list', () => {
	test("shows a member's role count on their detail panel", async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
		await withUI(browser, owner, async (page) => {
			// The count moved off the row and onto the panel when the rail
			// narrowed: the row now carries only the name, the subtitle and an
			// Owner/Pending badge. Selecting the member shows "Access" with the
			// number of bindings beside it, and the role itself listed below.
			await page.getByText(invitee.email).click();
			await expect(page.getByText(/^Access$/).first()).toBeVisible({ timeout: 10_000 });
			await expect(page.getByText(r1.name).first()).toBeVisible();
		});
	});

	test('shows only the Owner row when the workspace has no other members', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		await withUI(browser, owner, async (page) => {
			// The owner is always present (synthetic row pinned to top); no other
			// members means just the one row with the Owner badge.
			await expect(page.getByText(/^Owner$/).first()).toBeVisible({
				timeout: 10_000,
			});
			// Username appears in the user-dropdown header too; scope to the
			// member row (use .first()).
			await expect(page.getByText(owner.email).first()).toBeVisible();
			// The owner holds no bindings — the super-admin bypasses roles
			// entirely — so no role name is listed on their panel.
			await expect(page.getByText(/^No roles assigned\.$/).first()).toBeVisible();
		});
	});

	test('navigates to /workspace/members from the workspace settings tab', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await page.getByRole('link', { name: /^Members$/ }).click();
			await expectUrl(page, /\/workspace\/members$/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
