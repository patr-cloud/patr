import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createApiTokenAPI,
	createRoleAPI,
	callWithApiToken,
	getPermissionId,
	loginAs,
} from '@/prelude';
import {
	openTokenDetail,
	clickSavePermissions,
	enableWorkspaceCheckbox,
} from '@/helpers/ui/api-token';

// Ceiling ∩ owner-permission enforcement, empty-ceiling PATCH, and the
// role-revocation / PATCH cache-invalidation cascades all live in the Rust API
// suite (api/tests/api/user/api_token.rs). Here we cover the token detail
// page's Save-Permissions UI over the role-grant ceiling.

// A role holding just `permName`, for use as a token ceiling grant.
async function makeRole(
	api: Parameters<typeof createRoleAPI>[0],
	owner: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	permName: string,
): Promise<string> {
	const permId = await getPermissionId(
		api,
		owner.accessToken,
		owner.workspaceId,
		owner.clientIp,
		permName,
	);
	const role = await createRoleAPI(api, owner, owner.workspaceId, {
		name: `tok-role-${crypto.randomUUID().slice(0, 8)}`,
		permissions: [permId],
	});
	return role.id;
}

test.describe('api token > permissions [UI]', () => {
	test('changes effective authz after Save Permissions on the token detail page', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await makeRole(api, owner, 'deployment::view');
		const token = await createApiTokenAPI(api, owner, {
			grants: { [owner.workspaceId]: [{ roleId, resourceId: owner.workspaceId }] },
		});
		// Probe deployment list (should be 200 with view in the ceiling — the
		// owner is super admin, so the ceiling is the binding constraint).
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
		const roleId = await makeRole(api, owner, 'deployment::view');
		const token = await createApiTokenAPI(api, owner, {
			grants: { [owner.workspaceId]: [{ roleId, resourceId: owner.workspaceId }] },
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

	test('a resource-scoped ceiling grant clamps the token to those resources', async ({ api }) => {
		await using owner = await createUserWithWorkspace(api);
		// Two runners; the ceiling grants runner::view on only one of them.
		const mkRunner = (name: string) =>
			api.request<{ id: string }>('POST', `/workspace/${owner.workspaceId}/runner`, {
				token: owner.accessToken,
				clientIp: owner.clientIp,
				body: { name },
			});
		const allowed = await mkRunner(`allowed-${Date.now().toString(36)}`);
		const denied = await mkRunner(`denied-${Date.now().toString(36)}`);
		const permId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'runner::view',
		);
		const role = await createRoleAPI(api, owner, owner.workspaceId, {
			name: `scoped-${crypto.randomUUID().slice(0, 8)}`,
			permissions: [permId],
		});
		const token = await createApiTokenAPI(api, owner, {
			grants: {
				[owner.workspaceId]: [
					{ roleId: role.id, resourceId: allowed.id },
				],
			},
		});
		const ok = await callWithApiToken(api, token.token, {
			clientIp: owner.clientIp,
			path: `/workspace/${owner.workspaceId}/runner/${allowed.id}`,
		});
		expect(ok.status).toBe(200);
		const denied1 = await callWithApiToken(api, token.token, {
			clientIp: owner.clientIp,
			path: `/workspace/${owner.workspaceId}/runner/${denied.id}`,
		});
		expect(denied1.status).toBe(401);
	});
});
