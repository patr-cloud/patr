import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	addMemberToWorkspace,
	createRoleAPI,
	getPermissionId,
	loginAs,
} from '@/prelude';
import {
	openRoleDetail,
	fillRoleForm,
	openRoleUsersTab,
	clickClearAllPermissions,
	expectUnsavedChangesModal,
} from '@/helpers/ui/role';

// Role update at the API layer (rename, description, add/remove/replace
// permissions, empty-body/empty-perms rejection) lives in the Rust API suite
// (api/tests/api/workspace/rbac/mod.rs). Here we cover the edit-role UI.

async function makeRole(
	api: Parameters<typeof createRoleAPI>[0],
	user: Parameters<typeof createRoleAPI>[1] & { workspaceId: string; clientIp: string },
	name: string,
) {
	const viewId = await getPermissionId(
		api,
		user.accessToken,
		user.workspaceId,
		user.clientIp,
		'viewRoles',
	);
	return createRoleAPI(api, user, user.workspaceId, {
		name,
		description: 'initial-desc',
		permissions: { [viewId]: { permissionType: 'exclude', resources: [] } },
	});
}

test.describe('role > update [UI]', () => {
	test('exposes name and description fields on the edit-role UI', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const role = await makeRole(api, user, `fix-${Date.now().toString(36)}`);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRoleDetail(page, role.id);
			await expect(page.getByPlaceholder('Enter Name')).toBeVisible({ timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('disables Save Changes after Clear All wipes every permission', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const role = await makeRole(api, user, `empty-${Date.now().toString(36)}`);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRoleDetail(page, role.id);
			// Wait for the seeded permission to load (Save enabled) before clearing,
			// so we're not racing the query that populates the matrix.
			await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeEnabled({
				timeout: 10_000,
			});
			await clickClearAllPermissions(page);
			await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeDisabled();
		} finally {
			await context.close();
		}
	});

	// Navigation-blocking specs: tagged @racy so they run in the serial pass —
	// the router blocker + HMR make them sensitive to concurrent navigation.
	test('@racy warns about unsaved changes on leave, and Stay keeps you on the tab', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const roleName = `guard-${Date.now().toString(36)}`;
		const role = await makeRole(api, user, roleName);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRoleDetail(page, role.id);
			// Let the form seed from the server, then dirty it via the name field —
			// no matrix interaction needed to make isDirty true.
			await expect(page.getByPlaceholder('Enter Name')).toHaveValue(roleName, {
				timeout: 10_000,
			});
			await fillRoleForm(page, { name: `${roleName}-edited` });

			await openRoleUsersTab(page);
			await expectUnsavedChangesModal(page);

			await page.getByRole('button', { name: /^Stay$/ }).click();
			// Blocker reset — still on the edit tab with the edit still pending.
			await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeVisible();
			await expect(page.getByPlaceholder('Enter Name')).toHaveValue(`${roleName}-edited`);
		} finally {
			await context.close();
		}
	});

	test('@racy discards changes and navigates away when Leave is chosen', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const roleName = `leave-${Date.now().toString(36)}`;
		const role = await makeRole(api, user, roleName);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRoleDetail(page, role.id);
			await expect(page.getByPlaceholder('Enter Name')).toHaveValue(roleName, {
				timeout: 10_000,
			});
			await fillRoleForm(page, { name: `${roleName}-edited` });

			await openRoleUsersTab(page);
			await expectUnsavedChangesModal(page);

			await page.getByRole('button', { name: /^Leave$/ }).click();
			// Navigation proceeded to the Users tab (role has no members).
			await expect(page.getByText(/No users have been assigned this role yet/i)).toBeVisible({
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});

	test('preserves state when navigating between Edit Permissions and Users tabs', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const role = await makeRole(api, user, `tabs-${Date.now().toString(36)}`);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRoleDetail(page, role.id);
			await page.getByRole('link', { name: /^Users$/ }).click();
			await expect(page.getByText(/No users have been assigned this role yet/i)).toBeVisible({
				timeout: 10_000,
			});
			await page.getByRole('link', { name: /^Edit Permissions$/ }).click();
			await expect(page.getByRole('button', { name: /^Save Changes$/ })).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test('shows the empty Users tab when no users hold the role', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const role = await makeRole(api, user, `noUsers-${Date.now().toString(36)}`);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto(`/workspace/roles/${role.id}?tab=users`, {
				waitUntil: 'domcontentloaded',
			});
			await expect(page.getByText(/No users have been assigned this role yet/i)).toBeVisible({
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});

	test('lists assigned users with a count on the Users tab', async ({ api, browser }) => {
		await using owner = await createUserWithWorkspace(api);
		const role = await makeRole(api, owner, `withUsers-${Date.now().toString(36)}`);
		await using invitee = await createUserAccount(api);
		await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [role.id]);
		const context = await newContext(browser, owner.clientIp);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto(`/workspace/roles/${role.id}?tab=users`, {
				waitUntil: 'domcontentloaded',
			});
			await expect(page.getByText(invitee.email)).toBeVisible({ timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
