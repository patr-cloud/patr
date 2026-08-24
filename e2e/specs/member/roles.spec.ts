import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	addMemberToWorkspace,
	listRolesAPI,
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

// Member role-assignment at the API layer (set roles, empty→remove, nonexistent
// roleId, cross-workspace role) lives in the Rust API suite
// (api/tests/api/workspace/rbac/mod.rs). Here we cover the member roles editor UI.

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

test.describe('member > roles [UI]', () => {
	test('adds a role via the chip dropdown and persists it', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		const r2 = roles.find((r) => /Deployment: Viewer/i.test(r.name))!;
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
		await withUI(browser, owner, async (page) => {
			await page.getByText(`@${invitee.username}`).click();
			await clickEditRoles(page);
			await addRoleViaChipDropdown(page, r2.name);
			await saveMemberRoles(page);
			await expectToast(page, /Roles updated successfully/i);
		});
		const inviteeId = await getOwnUserId(api, invitee);
		const rows = await sql<{ role_id: string }>(
			`SELECT rb.role_id
			   FROM role_binding rb
			   JOIN actor a ON a.id = rb.actor_id
			  WHERE rb.workspace_id = $1 AND a.user_id = $2`,
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
			await page.getByText(`@${invitee.username}`).click();
			await clickEditRoles(page);
			await removeRoleChip(page, r2.name);
			await saveMemberRoles(page);
			await expectToast(page, /Roles updated successfully/i);
		});
		const inviteeId = await getOwnUserId(api, invitee);
		const rows = await sql<{ role_id: string }>(
			`SELECT rb.role_id
			   FROM role_binding rb
			   JOIN actor a ON a.id = rb.actor_id
			  WHERE rb.workspace_id = $1 AND a.user_id = $2`,
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
			await page.getByText(`@${invitee.username}`).click();
			await clickEditRoles(page);
			await removeRoleChip(page, r1.name);
			await cancelMemberRolesEdit(page);
		});
		const inviteeId = await getOwnUserId(api, invitee);
		// Membership row: workspace_user is one row per member now; the grant
		// itself lives in role_binding.
		const rows = await sql(
			`SELECT rb.id
			   FROM role_binding rb
			   JOIN actor a ON a.id = rb.actor_id
			  WHERE rb.workspace_id = $1 AND a.user_id = $2`,
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
			await page.getByText(`@${invitee.username}`).click();
			await clickEditRoles(page);
			const link = page.getByRole('link', { name: /create a new role/i });
			await expect(link).toHaveAttribute('href', '/workspace/roles/new');
		});
	});
});
