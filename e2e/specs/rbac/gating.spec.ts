import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	getPermissionId,
	loginAs,
} from '@/prelude';

test.describe('rbac > app-wide UI gating (useIsAllowed)', () => {
	test('hides Create Deployment from a member without deployment::create', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const viewId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'deployment::view',
		);
		await using b = await createSecondMemberWithRole(api, owner, {
			[viewId]: { permissionType: 'exclude', resources: [] },
		});
		const context = await newContext(browser, b.clientIp);
		await loginAs(context, b, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/deployments', { waitUntil: 'domcontentloaded' });
			await expect(page.getByRole('button', { name: /^Create Deployment$/ })).toBeHidden({
				timeout: 10_000,
			});
			await expect(page.getByRole('link', { name: /^Create Deployment$/ })).toBeHidden();
		} finally {
			await context.close();
		}
	});

	test('shows Create Deployment to a member with deployment::create', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const viewId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'deployment::view',
		);
		const createId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'deployment::create',
		);
		await using b = await createSecondMemberWithRole(api, owner, {
			[viewId]: { permissionType: 'exclude', resources: [] },
			[createId]: { permissionType: 'exclude', resources: [] },
		});
		const context = await newContext(browser, b.clientIp);
		await loginAs(context, b, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/deployments', { waitUntil: 'domcontentloaded' });
			// Either button or link with Create Deployment is visible.
			const visible = await Promise.race([
				page
					.getByRole('link', { name: /Create Deployment/i })
					.first()
					.waitFor({ timeout: 5_000 })
					.then(() => true)
					.catch(() => false),
				page
					.getByRole('button', { name: /Create Deployment/i })
					.first()
					.waitFor({ timeout: 5_000 })
					.then(() => true)
					.catch(() => false),
			]);
			expect(visible).toBe(true);
		} finally {
			await context.close();
		}
	});

	test('hides Create Runner from a member without runner::create', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const viewId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'runner::view',
		);
		await using b = await createSecondMemberWithRole(api, owner, {
			[viewId]: { permissionType: 'exclude', resources: [] },
		});
		const context = await newContext(browser, b.clientIp);
		await loginAs(context, b, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/runners', { waitUntil: 'domcontentloaded' });
			await expect(page.getByRole('button', { name: /Create Runner/i })).toBeHidden({
				timeout: 10_000,
			});
			await expect(page.getByRole('link', { name: /Create Runner/i })).toBeHidden();
		} finally {
			await context.close();
		}
	});

	test('hides Add Member from a member with only viewRoles on /workspace/members', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const viewId = await getPermissionId(
			api,
			owner.accessToken,
			owner.workspaceId,
			owner.clientIp,
			'viewRoles',
		);
		await using b = await createSecondMemberWithRole(api, owner, {
			[viewId]: { permissionType: 'exclude', resources: [] },
		});
		const context = await newContext(browser, b.clientIp);
		await loginAs(context, b, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			await expect(
				page.getByRole('button', { name: /^(Add Member|Adding\.\.\.)$/ }),
			).toBeHidden({
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});
});
