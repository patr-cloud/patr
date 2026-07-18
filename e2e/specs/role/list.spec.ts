import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createRoleAPI,
	getPermissionId,
	loginAs,
} from '@/prelude';
import { openRolesList } from '@/helpers/ui/role';

// The default-roles seeding count is asserted in the Rust API suite
// (api/tests/api/workspace/rbac/mod.rs). Here we cover the roles list UI.

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

test.describe('role > list [UI]', () => {
	test('lists a newly-created role with its name and description', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = `list-${Date.now().toString(36)}`;
		await createRoleAPI(api, user, user.workspaceId, {
			name,
			description: 'my desc',
			permissions: {
				[await getPermissionId(
					api,
					user.accessToken,
					user.workspaceId,
					user.clientIp,
					'viewRoles',
				)]: { permissionType: 'exclude', resources: [] },
			},
		});
		await withUI(browser, user, async (page) => {
			await openRolesList(page);
			await expect(page.getByRole('row').filter({ hasText: name })).toBeVisible({
				timeout: 10_000,
			});
			await expect(page.getByText('my desc')).toBeVisible();
		});
	});
});
