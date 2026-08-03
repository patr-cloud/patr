import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	getPermissionId,
	loginAs,
	getOwnUserId,
} from '@/prelude';
import { sql } from '@/helpers/db';
import { openWorkspaceSettings } from '@/helpers/ui/workspace';

async function membershipRows(workspaceId: string, userId: string) {
	return sql('SELECT user_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2', [
		workspaceId,
		userId,
	]);
}

// A plain member of someone else's workspace, with enough permission to load
// the settings page.
async function memberOf(
	api: Parameters<typeof getPermissionId>[0],
	owner: Awaited<ReturnType<typeof createUserWithWorkspace>>,
) {
	const viewId = await getPermissionId(
		api,
		owner.accessToken,
		owner.workspaceId,
		owner.clientIp,
		'viewRoles',
	);
	return createSecondMemberWithRole(api, owner, {
		[viewId]: { permissionType: 'exclude', resources: [] },
	});
}

test.describe('workspace > leave [UI]', () => {
	test('a member leaves and loses access to the workspace', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using member = await memberOf(api, owner);
		const memberId = await getOwnUserId(api, member);

		expect(await membershipRows(owner.workspaceId, memberId)).toHaveLength(1);

		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openWorkspaceSettings(page);

			// Two-step: the outlined button arms the confirm.
			await page.getByRole('button', { name: /^Leave workspace$/ }).click();
			await page.getByRole('button', { name: /^(Confirm leave|Leaving\.\.\.)$/ }).click();

			await expect
				.poll(async () => membershipRows(owner.workspaceId, memberId), {
					timeout: 15_000,
				})
				.toHaveLength(0);
		} finally {
			await context.close();
		}
	});

	test('the leave control is hidden from the workspace owner', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);

		const context = await newContext(browser, owner.clientIp);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openWorkspaceSettings(page);
			await expect(page.getByRole('button', { name: /^Leave workspace$/ })).toBeHidden({
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});

	test('leaving can be cancelled without dropping membership', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using member = await memberOf(api, owner);
		const memberId = await getOwnUserId(api, member);

		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openWorkspaceSettings(page);
			await page.getByRole('button', { name: /^Leave workspace$/ }).click();
			await page.getByRole('button', { name: /^Cancel$/ }).click();

			await expect(page.getByRole('button', { name: /^Leave workspace$/ })).toBeVisible();
			expect(await membershipRows(owner.workspaceId, memberId)).toHaveLength(1);
		} finally {
			await context.close();
		}
	});
});
