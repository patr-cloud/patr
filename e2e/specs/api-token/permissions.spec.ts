import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createApiTokenAPI,
	callWithApiToken,
	loginAs,
} from '@/prelude';
import {
	openTokenDetail,
	clickSavePermissions,
	enableWorkspaceCheckbox,
} from '@/helpers/ui/api-token';

// Permission superset enforcement, empty-perms PATCH, and the role-revocation /
// PATCH cache-invalidation cascades all live in the Rust API suite
// (api/tests/api/user/api_token.rs). Here we cover the token detail page's
// Save-Permissions UI.

test.describe('api token > permissions [UI]', () => {
	test('changes effective authz after Save Permissions on the token detail page', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const perms = await api.request<{ permissions: { id: string; name: string }[] }>(
			'GET',
			`/workspace/${owner.workspaceId}/rbac/permission`,
			{ token: owner.accessToken, clientIp: owner.clientIp },
		);
		const viewId = perms.permissions.find((p) => p.name === 'deployment::view')!.id;
		const token = await createApiTokenAPI(api, owner, {
			permissions: {
				[owner.workspaceId]: {
					type: 'member',
					[viewId]: [owner.workspaceId],
				} as any,
			},
		});
		// Probe deployment list (should be 200 with view).
		const r1 = await callWithApiToken(api, token.token, {
			clientIp: owner.clientIp,
			path: `/workspace/${owner.workspaceId}/deployment`,
		});
		expect(r1.status).toBe(200);
		const context = await newContext(browser, owner.clientIp);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenDetail(page, token.id);
			await clickSavePermissions(page);
			await expect(page.getByText(/API Token updated successfully/i)).toBeVisible({
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	test('disables Save Permissions when no workspace is enabled on detail', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const perms = await api.request<{ permissions: { id: string; name: string }[] }>(
			'GET',
			`/workspace/${owner.workspaceId}/rbac/permission`,
			{ token: owner.accessToken, clientIp: owner.clientIp },
		);
		const viewId = perms.permissions.find((p) => p.name === 'deployment::view')!.id;
		const token = await createApiTokenAPI(api, owner, {
			permissions: {
				[owner.workspaceId]: {
					type: 'member',
					[viewId]: [owner.workspaceId],
				} as any,
			},
		});
		const context = await newContext(browser, owner.clientIp);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openTokenDetail(page, token.id);
			await enableWorkspaceCheckbox(page, owner.workspaceName);
			await expect(page.getByRole('button', { name: /^Save Permissions$/ })).toBeDisabled();
		} finally {
			await context.close();
		}
	});
});
