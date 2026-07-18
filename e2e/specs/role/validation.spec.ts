import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import {
	openCreateRolePage,
	fillRoleForm,
	addWorkspaceLevelPermission,
	submitCreateRole,
	expectToast,
} from '@/helpers/ui/role';

// Role name/permission validation at the API layer (length bounds, charset,
// duplicate→409, same-name-cross-workspace, empty-permissions/empty-body PATCH)
// lives in the Rust API suite (api/tests/api/workspace/rbac/mod.rs). Here we
// cover the create form's client-side guards.

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

test.describe('role > validation [UI]', () => {
	test('shows a client toast when role name is empty', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withUI(browser, user, async (page) => {
			await openCreateRolePage(page);
			await addWorkspaceLevelPermission(page, 'View Roles');
			await submitCreateRole(page);
			await expectToast(page, /Please enter a role name/i);
		});
	});

	test('shows a client toast when no permission is selected', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withUI(browser, user, async (page) => {
			await openCreateRolePage(page);
			await fillRoleForm(page, { name: 'something' });
			await submitCreateRole(page);
			await expectToast(page, /Please select at least one permission/i);
		});
	});
});
