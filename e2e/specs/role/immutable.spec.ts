import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createRoleAPI,
	getPermissionId,
	listRolesAPI,
	loginAs,
} from '@/prelude';
import { openRolesList, openRoleDetail } from '@/helpers/ui/role';

// Immutable (seeded default) roles: visible, grantable, but not editable or
// deletable. API-level enforcement (403 roleIsImmutable on PATCH/DELETE) lives
// in the Rust API suite (api/tests/api/workspace/rbac/mod.rs); here we cover
// the dashboard's read-only treatment.

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

test.describe('role > immutable [UI]', () => {
	test('marks built-in roles on the list and withholds their delete button', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const roles = await listRolesAPI(api, user, user.workspaceId);
		const builtIn = roles.find((r) => r.isImmutable)!;
		expect(builtIn).toBeTruthy();
		await withUI(browser, user, async (page) => {
			await openRolesList(page);
			const row = page.getByRole('row').filter({ hasText: builtIn.name }).first();
			await expect(row.getByText(/^Built-in$/)).toBeVisible({ timeout: 10_000 });
			await expect(row.getByRole('button', { name: /Delete role/i })).toHaveCount(0);
		});
	});

	test('keeps the delete button on custom roles', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const viewId = await getPermissionId(
			api,
			user.accessToken,
			user.workspaceId,
			user.clientIp,
			'viewRoles',
		);
		const roleName = `custom-${Date.now().toString(36)}`;
		await createRoleAPI(api, user, user.workspaceId, {
			name: roleName,
			permissions: [viewId],
		});
		await withUI(browser, user, async (page) => {
			await openRolesList(page);
			const row = page.getByRole('row').filter({ hasText: roleName }).first();
			await expect(row.getByRole('button', { name: /Delete role/i })).toBeVisible({
				timeout: 10_000,
			});
			await expect(row.getByText(/^Built-in$/)).toHaveCount(0);
		});
	});

	test('renders a built-in role read-only on the detail page', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const roles = await listRolesAPI(api, user, user.workspaceId);
		const builtIn = roles.find((r) => r.isImmutable)!;
		await withUI(browser, user, async (page) => {
			await openRoleDetail(page, builtIn.id);
			await expect(
				page.getByText(/This is a built-in role and cannot be edited or deleted\./),
			).toBeVisible({ timeout: 10_000 });
			await expect(page.getByPlaceholder('Enter Name')).toBeDisabled();
			await expect(page.getByRole('button', { name: /^Save Changes$/ })).toHaveCount(0);
			await expect(page.getByRole('button', { name: /^Clear All$/ })).toHaveCount(0);
		});
	});
});
