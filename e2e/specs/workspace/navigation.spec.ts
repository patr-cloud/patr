import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	createUserWithWorkspaces,
	addMemberToWorkspace,
	loginAs,
	sql,
	expectUrl,
} from '@/prelude';
import {
	openWorkspaceSettings,
	openWorkspaceSwitcher,
	clickSwitcherWorkspace,
	getLastWorkspaceIdCookie,
	waitForActiveWorkspaceCookie,
} from '@/helpers/ui/workspace';

async function withTwoWorkspaces(
	browser: import('@playwright/test').Browser,
	api: import('@/prelude').ApiClient,
	fn: (
		page: import('@playwright/test').Page,
		context: import('@playwright/test').BrowserContext,
		user: Awaited<ReturnType<typeof createUserWithWorkspaces>>,
	) => Promise<void>,
) {
	const suffix = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
	await using user = await createUserWithWorkspaces(api, [`alpha-${suffix}`, `beta-${suffix}`]);
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaces[0].id });
	const page = await context.newPage();
	try {
		await fn(page, context, user);
	} finally {
		await context.close();
	}
}

test.describe('workspace > navigation @racy', () => {
	test('refetches workspace-scoped queries when switching workspaces', async ({
		browser,
		api,
	}) => {
		await withTwoWorkspaces(browser, api, async (page, _ctx, user) => {
			// Add another member to alpha so its members list is non-empty.
			const inviteeSuffix = Math.random().toString(36).slice(2, 8);
			await using invitee = await createUserAccount(api);
			// Look up a role id from alpha's seeded defaults.
			const roles = await api.request<{ roles: { id: string; name: string }[] }>(
				'GET',
				`/workspace/${user.workspaces[0].id}/rbac/role?page=0&count=100`,
				{ token: user.accessToken, clientIp: user.clientIp },
			);
			const viewerRole = roles.roles.find((r) => /Viewer/.test(r.name));
			expect(viewerRole).toBeTruthy();
			await addMemberToWorkspace(api, user, user.workspaces[0].id, invitee, [viewerRole!.id]);

			// Navigate to members (active workspace is alpha).
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			// Alpha has 1 invitee in workspace_user (owner not in list).
			await expect(page.getByText(`@${invitee.username}`).first()).toBeVisible({
				timeout: 10_000,
			});

			// Switch to beta — members list should now show no invitees. The reload
			// reads the active workspace from the cookie, so wait for the switch to
			// commit before navigating (the click alone doesn't guarantee it).
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await waitForActiveWorkspaceCookie(page, user.workspaces[1].id);
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			await expect(page.getByText(`@${invitee.username}`).first()).toBeHidden({
				timeout: 10_000,
			});

			// Switch back to alpha; invitee should reappear.
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[0].name);
			await waitForActiveWorkspaceCookie(page, user.workspaces[0].id);
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			await expect(page.getByText(`@${invitee.username}`).first()).toBeVisible({
				timeout: 10_000,
			});
			// Touch suffix to silence unused-var lint.
			expect(inviteeSuffix).toBeTruthy();
		});
	});

	test('refetches workspace info on the settings page when switching workspaces', async ({
		browser,
		api,
	}) => {
		await withTwoWorkspaces(browser, api, async (page, _ctx, user) => {
			await openWorkspaceSettings(page);
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaces[0].name, {
				timeout: 10_000,
			});
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaces[1].name, {
				timeout: 10_000,
			});
		});
	});

	test('honours the prior lastWorkspaceId cookie when deep-linking to /workspace', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspaces(api, [
			`one-${Date.now().toString(36)}`,
			`two-${Date.now().toString(36)}`,
		]);
		const context = await newContext(browser, user.clientIp);
		// Pre-set cookie to point at the second workspace.
		await loginAs(context, user, { workspaceId: user.workspaces[1].id });
		const page = await context.newPage();
		try {
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaces[1].name, {
				timeout: 10_000,
			});
		} finally {
			await context.close();
		}
	});

	test('redirects the user to /onboard when all workspaces are soft-deleted', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expect(page.locator('#workspace-name')).not.toHaveValue('', {
				timeout: 10_000,
			});
			// Soft-delete the workspace so list_user_workspaces no longer includes
			// it. Bypassing the DELETE /workspace endpoint to avoid the audit_log
			// FK quirk; the listing query filters on workspace.deleted IS NULL.
			await sql(`UPDATE workspace SET deleted = NOW() WHERE id = $1`, [user.workspaceId]);
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/onboard/, { timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('preserves the active workspace across browser back navigation', async ({
		browser,
		api,
	}) => {
		await withTwoWorkspaces(browser, api, async (page, _ctx, user) => {
			await openWorkspaceSettings(page);
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await waitForActiveWorkspaceCookie(page, user.workspaces[1].id);
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			await page.goBack();
			await expectUrl(page, /\/workspace$/);
			// Active workspace should still be beta.
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaces[1].name, {
				timeout: 10_000,
			});
		});
	});

	test('inherits the same active workspace in a second tab of the same context', async ({
		browser,
		api,
	}) => {
		await withTwoWorkspaces(browser, api, async (page, context, user) => {
			await openWorkspaceSettings(page);
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			// Same race as the reload sites: the one-shot cookie read below can beat
			// the page's cookie write. Wait for the commit first.
			await waitForActiveWorkspaceCookie(page, user.workspaces[1].id);
			const cookieId = await getLastWorkspaceIdCookie(context);
			expect(cookieId).toBe(user.workspaces[1].id);
			const page2 = await context.newPage();
			await page2.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expect(page2.locator('#workspace-name')).toHaveValue(user.workspaces[1].name, {
				timeout: 10_000,
			});
		});
	});

	test('does not auto-propagate workspace switches to other open tabs', async ({
		browser,
		api,
	}) => {
		await withTwoWorkspaces(browser, api, async (page, context, user) => {
			await openWorkspaceSettings(page);
			// Open tab2 first while still on alpha.
			const page2 = await context.newPage();
			await page2.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expect(page2.locator('#workspace-name')).toHaveValue(user.workspaces[0].name, {
				timeout: 10_000,
			});
			// Switch on tab1 and wait for it to commit — otherwise the negative
			// assert below could pass vacuously before the switch even happened.
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await waitForActiveWorkspaceCookie(page, user.workspaces[1].id);
			// Tab2 has not navigated/reloaded — assert it still shows alpha.
			await expect(page2.locator('#workspace-name')).toHaveValue(user.workspaces[0].name);
		});
	});

	test('keeps the chosen workspace after a reload', async ({ browser, api }) => {
		await withTwoWorkspaces(browser, api, async (page, _ctx, user) => {
			await openWorkspaceSettings(page);
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await waitForActiveWorkspaceCookie(page, user.workspaces[1].id);
			// page.reload() can hang against Vinxi dev (see auth specs note); use goto.
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaces[1].name, {
				timeout: 10_000,
			});
		});
	});
});
