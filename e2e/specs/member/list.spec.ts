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
	test('renders a role-count badge on each member row', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
		await withUI(browser, owner, async (page) => {
			// JSX renders `${n}&nbsp; role(s)` (non-breaking space + newline).
			await expect(page.getByText(/^1\s+role$/).first()).toBeVisible({
				timeout: 10_000,
			});
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
			// No "1 role" / "2 roles" badges should appear — owner has no roles.
			await expect(page.getByText(/^\d+\s+roles?$/)).toHaveCount(0);
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
