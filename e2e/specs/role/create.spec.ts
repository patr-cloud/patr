import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	getRoleAPI,
	listRolesAPI,
	loginAs,
	expectUrl,
} from '@/prelude';
import {
	openRolesList,
	openCreateRolePage,
	fillRoleForm,
	addWorkspaceLevelPermission,
	submitCreateRole,
	expectToast,
} from '@/helpers/ui/role';

// Role create at the API layer (include/exclude scope, multiple permissions,
// name/duplicate/length validation) lives in the Rust API suite
// (api/tests/api/workspace/rbac/mod.rs). Here we cover the create form.

async function withUI(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('role > create', () => {
	test('creates a workspace-scoped role with a single permission via UI', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const roleName = `e2e-role-${Date.now().toString(36)}`;
		await withUI(browser, user, async (page) => {
			await openCreateRolePage(page);
			await fillRoleForm(page, { name: roleName, description: 'create test' });
			await addWorkspaceLevelPermission(page, 'View Roles');
			await submitCreateRole(page);
			await expectToast(page, /Role created successfully/i);
		});
		const roles = await listRolesAPI(api, user, user.workspaceId);
		const r = roles.find((r) => r.name === roleName);
		expect(r).toBeTruthy();
		const detail = await getRoleAPI(api, user, user.workspaceId, r!.id);
		expect(detail.permissions.length).toBeGreaterThanOrEqual(1);
	});

	test('creates a workspace-level modifyRoles role via UI', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const roleName = `ws-${Date.now().toString(36)}`;
		await withUI(browser, user, async (page) => {
			await openCreateRolePage(page);
			await fillRoleForm(page, { name: roleName });
			await addWorkspaceLevelPermission(page, 'Modify Roles');
			await submitCreateRole(page);
			await expectToast(page, /Role created successfully/i);
		});
		const roles = await listRolesAPI(api, user, user.workspaceId);
		expect(roles.find((r) => r.name === roleName)).toBeTruthy();
	});

	test('routes to the new-role page from the header link', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withUI(browser, user, async (page) => {
			await openRolesList(page);
			await page.getByRole('link', { name: /^Create New Role$/ }).click();
			await expectUrl(page, /\/workspace\/roles\/new$/, { timeout: 10_000 });
		});
	});
});

test.describe('role > description > inline validation', () => {
	test('rejects HTML in description with inline error', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withUI(browser, user, async (page) => {
			await openCreateRolePage(page);
			await fillRoleForm(page, {
				name: 'role-' + Date.now().toString(36),
				description: '<script>x</script>',
			});
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/rbac/role') && req.method() === 'POST') {
					fired = true;
				}
			});
			await submitCreateRole(page);
			await expect(
				page.getByText(/Description cannot contain <, >, &, or control characters/),
			).toBeVisible({ timeout: 5_000 });
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
		});
	});
});
