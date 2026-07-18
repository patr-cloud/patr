import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	addMemberToWorkspace,
	listRolesAPI,
	loginAs,
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

test.describe('member > detail', () => {
	test('renders role chips with names for the selected member', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
		await withUI(browser, owner, async (page) => {
			// The Owner row is first by default; click the invitee row to select it.
			await page.getByText(`@${invitee.username}`).click();
			await expect(page.getByText(r1.name).first()).toBeVisible({ timeout: 10_000 });
		});
	});
});
