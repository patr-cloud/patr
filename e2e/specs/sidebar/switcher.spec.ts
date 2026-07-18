import {
	test,
	expect,
	newContext,
	createUserWithWorkspaces,
	loginAs,
	expectUrl,
	VINXI_DEV_URL,
} from '@/prelude';
import {
	openWorkspaceSettings,
	openWorkspaceSwitcher,
	closeWorkspaceSwitcher,
	clickSwitcherWorkspace,
	clickSwitcherCreateNew,
	getActiveSwitcherWorkspaceName,
	listSwitcherWorkspaceNames,
	getLastWorkspaceIdCookie,
} from '@/helpers/ui/workspace';

async function withThreeWorkspaces(
	browser: import('@playwright/test').Browser,
	api: import('@/prelude').ApiClient,
	fn: (
		page: import('@playwright/test').Page,
		context: import('@playwright/test').BrowserContext,
		user: Awaited<ReturnType<typeof createUserWithWorkspaces>>,
	) => Promise<void>,
) {
	const suffix = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
	const names = [`alpha-${suffix}`, `beta-${suffix}`, `gamma-${suffix}`];
	await using user = await createUserWithWorkspaces(api, names);
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaces[0].id });
	const page = await context.newPage();
	try {
		// Land on settings page (any _workspaced page works; this one shows the
		// sidebar deterministically).
		await openWorkspaceSettings(page);
		await fn(page, context, user);
	} finally {
		await context.close();
	}
}

test.describe('sidebar > workspace switcher', () => {
	test('shows the current workspace name in the switcher header', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page, _ctx, user) => {
			// Wait for workspace info to load; the trigger should now show alpha.
			await expect(page.getByText(user.workspaces[0].name).first()).toBeVisible({
				timeout: 10_000,
			});
			const active = await getActiveSwitcherWorkspaceName(page);
			expect(active).toBe(user.workspaces[0].name);
		});
	});

	test('lists every workspace the user belongs to when opened', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page, _ctx, user) => {
			await openWorkspaceSwitcher(page);
			const names = await listSwitcherWorkspaceNames(page);
			expect(names).toEqual(expect.arrayContaining(user.workspaces.map((w) => w.name)));
		});
	});

	test('renders the "Workspaces" panel heading', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page) => {
			await openWorkspaceSwitcher(page);
			await expect(page.getByText('Workspaces', { exact: true })).toBeVisible();
		});
	});

	test('links the CREATE WORKSPACE footer to /workspace/new', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page) => {
			await openWorkspaceSwitcher(page);
			const link = page.getByRole('link', { name: /^CREATE WORKSPACE$/ });
			await expect(link).toHaveAttribute('href', '/workspace/new');
		});
	});

	test('closes the panel, updates the header, and writes the cookie when switching', async ({
		browser,
		api,
	}) => {
		await withThreeWorkspaces(browser, api, async (page, context, user) => {
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[1].name);
			await expect(page.getByText('Workspaces', { exact: true })).toBeHidden({
				timeout: 5_000,
			});
			const active = await getActiveSwitcherWorkspaceName(page);
			expect(active).toBe(user.workspaces[1].name);
			const cookieId = await getLastWorkspaceIdCookie(context);
			expect(cookieId).toBe(user.workspaces[1].id);
		});
	});

	test('closes the panel without changing state when clicking the current workspace', async ({
		browser,
		api,
	}) => {
		await withThreeWorkspaces(browser, api, async (page, context, user) => {
			await openWorkspaceSwitcher(page);
			await clickSwitcherWorkspace(page, user.workspaces[0].name);
			await expect(page.getByText('Workspaces', { exact: true })).toBeHidden({
				timeout: 5_000,
			});
			const cookieId = await getLastWorkspaceIdCookie(context);
			expect(cookieId).toBe(user.workspaces[0].id);
		});
	});

	test('navigates to /workspace/new when CREATE WORKSPACE is clicked', async ({
		browser,
		api,
	}) => {
		await withThreeWorkspaces(browser, api, async (page) => {
			await openWorkspaceSwitcher(page);
			await clickSwitcherCreateNew(page);
			await expectUrl(page, /\/workspace\/new$/, { timeout: 10_000 });
		});
	});

	test('closes the switcher panel on click outside', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page) => {
			await openWorkspaceSwitcher(page);
			await closeWorkspaceSwitcher(page);
			await expect(page.getByText('Workspaces', { exact: true })).toBeHidden();
		});
	});

	test('navigates to /workspace when the settings gear is clicked', async ({ browser, api }) => {
		await withThreeWorkspaces(browser, api, async (page) => {
			// Already on /workspace from setup; navigate to /workspace/members and use gear to come back.
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			// The gear icon is a RouterLink to /workspace; locate by href.
			await page.locator('a[href="/workspace"]').first().click();
			await expectUrl(page, /\/workspace$/, { timeout: 10_000 });
		});
	});

	test('writes the lastWorkspaceId cookie with sameSite=Strict, non-HttpOnly, ~7-day expiry', async ({
		browser,
		api,
	}) => {
		await withThreeWorkspaces(browser, api, async (page, context) => {
			await openWorkspaceSwitcher(page);
			// Trigger a write by switching to second workspace.
			await page
				.locator('button')
				.filter({ hasText: /-/ })
				.nth(1)
				.click()
				.catch(() => undefined);
			const cookies = await context.cookies(VINXI_DEV_URL);
			const c = cookies.find((c) => c.name === 'lastWorkspaceId');
			expect(c).toBeTruthy();
			expect(c!.sameSite).toBe('Strict');
			expect(c!.httpOnly).toBe(false);
			// ~7-day expiry; allow ±1 day slack.
			const now = Date.now() / 1000;
			const sevenDays = 60 * 60 * 24 * 7;
			const slack = 60 * 60 * 24;
			expect(c!.expires).toBeGreaterThan(now + sevenDays - slack);
			expect(c!.expires).toBeLessThan(now + sevenDays + slack);
		});
	});

	test('reflects an API-side workspace create once the layout refetches', async ({
		browser,
		api,
	}) => {
		await withThreeWorkspaces(browser, api, async (page, _ctx, user) => {
			// Create a fourth workspace via API while the page is open.
			const fourthName = `delta-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
			await api.request('POST', '/workspace', {
				token: user.accessToken,
				clientIp: user.clientIp,
				body: { name: fourthName },
			});
			// Force a refetch via navigation (a guarded route remounts the layout).
			await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await openWorkspaceSwitcher(page);
			const names = await listSwitcherWorkspaceNames(page);
			expect(names).toEqual(expect.arrayContaining([fourthName]));
		});
	});
});
