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
import {
	openMembersPage,
	searchUser,
	selectUserFromSearch,
	openRolesDropdown,
	toggleRoleOption,
	submitAddMember,
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

test.describe('member > add', () => {
	test('triggers user search after typing 3 characters', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		await withUI(browser, owner, async (page) => {
			const reqPromise = page.waitForRequest(
				(r) => r.url().includes('/user/search?query=') && r.method() === 'GET',
				{ timeout: 10_000 },
			);
			await searchUser(page, invitee.username.slice(0, 5));
			await reqPromise;
		});
	});

	test('adds a member with a single role via UI', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		await withUI(browser, owner, async (page) => {
			await searchUser(page, invitee.username);
			await selectUserFromSearch(page, invitee.username);
			await openRolesDropdown(page);
			await toggleRoleOption(page, viewerRole.name);
			await submitAddMember(page);
			await expectToast(page, /User added successfully/i);
			await expect(page.getByText(`@${invitee.username}`).first()).toBeVisible({
				timeout: 10_000,
			});
		});
	});

	test('adds a member with multiple roles via UI', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		const r2 = roles.find((r) => /Deployment: Viewer/i.test(r.name))!;
		await withUI(browser, owner, async (page) => {
			await searchUser(page, invitee.username);
			await selectUserFromSearch(page, invitee.username);
			await openRolesDropdown(page);
			await toggleRoleOption(page, r1.name);
			await toggleRoleOption(page, r2.name);
			await submitAddMember(page);
			await expectToast(page, /User added successfully/i);
		});
		const { sql } = await import('@/helpers/db');
		const { getOwnUserId } = await import('@/helpers/user');
		const inviteeId = await getOwnUserId(api, invitee);
		const rows = await sql<{ role_id: string }>(
			`SELECT role_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2`,
			[owner.workspaceId, inviteeId],
		);
		expect(rows.length).toBe(2);
	});

	test('clears the add-member form after a successful add', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		await withUI(browser, owner, async (page) => {
			await searchUser(page, invitee.username);
			await selectUserFromSearch(page, invitee.username);
			await openRolesDropdown(page);
			await toggleRoleOption(page, r1.name);
			await submitAddMember(page);
			await expectToast(page, /User added successfully/i);
			await expect(page.locator('input[placeholder="Add roles..."]').first()).toBeVisible({
				timeout: 10_000,
			});
		});
	});

	test('shows an error toast when an add-member call fails', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const r1 = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;
		// Pre-add the user so the second attempt fails (or is idempotent).
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [r1.id]);
		await withUI(browser, owner, async (page) => {
			await searchUser(page, invitee.username);
			await selectUserFromSearch(page, invitee.username);
			await openRolesDropdown(page);
			await toggleRoleOption(page, r1.name);
			await submitAddMember(page);
			const hadToast = await Promise.race([
				page
					.getByText(/Failed to add user/i)
					.first()
					.waitFor({ timeout: 5_000 })
					.then(() => true)
					.catch(() => false),
				page
					.getByText(/User added successfully/i)
					.first()
					.waitFor({ timeout: 5_000 })
					.then(() => true)
					.catch(() => false),
			]);
			expect(hadToast).toBe(true);
		});
	});
});
