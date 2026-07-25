import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	loginAs,
	expectUrl,
} from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import {
	openFirstWorkspaceScreen,
	fillWorkspaceName,
	submitFirstWorkspace,
	firstWorkspaceButton,
	expectFirstWorkspaceScreen,
	expectToast,
	getLastWorkspaceIdCookie,
} from '@/helpers/ui/workspace';

const VALID = () => `wks-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

// Runs `fn` against the zero-workspace create screen: a fresh account with no
// workspace, logged in, sitting on `/` where the _workspaced layout renders the
// inline create-first-workspace screen in place of the dashboard.
async function onFirstWorkspaceScreen(
	browser: import('@playwright/test').Browser,
	user: { accessToken: string; refreshToken: string; clientIp: string },
	fn: (
		page: import('@playwright/test').Page,
		context: import('@playwright/test').BrowserContext,
	) => Promise<void>,
) {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user as any);
	const page = await context.newPage();
	try {
		await openFirstWorkspaceScreen(page);
		await fn(page, context);
	} finally {
		await context.close();
	}
}

test.describe('workspace setup > route guards', () => {
	test('redirects unauthenticated visits to the dashboard to /login', async ({ browser }) => {
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await page.goto('/', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('shows the create-workspace screen to a zero-workspace user after login', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		const page = await context.newPage();
		try {
			await openLoginPage(page);
			await fillLoginForm(page, { userId: user.username, password: user.password });
			await submitLogin(page);
			await waitForLoggedIn(page);
			// No separate /onboard route — the dashboard renders the inline create
			// screen in place of the page, and the URL stays at the root.
			await expectFirstWorkspaceScreen(page);
			await expectUrl(page, /\/$/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('shows the dashboard (not the create screen) to a user who has a workspace', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await page.goto('/', { waitUntil: 'domcontentloaded' });
			await expect(page.getByText('Quick Actions')).toBeVisible({ timeout: 10_000 });
			await expect(page.locator('#workspace-name')).toBeHidden();
		} finally {
			await context.close();
		}
	});
});

test.describe('workspace setup > happy path', () => {
	test('creates the first workspace and swaps in the dashboard', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page, context) => {
			const name = VALID();
			await fillWorkspaceName(page, name);
			const respPromise = page.waitForResponse(
				(r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
				{ timeout: 30_000 },
			);
			await submitFirstWorkspace(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(true);
			await expectToast(page, /Workspace created successfully/i);
			// No navigation: creating the workspace invalidates the workspaces
			// query, and the layout reactively swaps the create screen for the
			// dashboard once the refetch lands.
			await expect(page.getByText('Quick Actions')).toBeVisible({ timeout: 10_000 });
			await expect(page.locator('#workspace-name')).toBeHidden();
			const cookieId = await getLastWorkspaceIdCookie(context);
			expect(cookieId).toBeTruthy();
		});
	});
});

test.describe('workspace setup > validation', () => {
	async function expectNoCreateRequest(
		page: import('@playwright/test').Page,
		interaction: () => Promise<void>,
	): Promise<void> {
		let fired = false;
		page.on('request', (req) => {
			if (req.url().endsWith('/api/workspace') && req.method() === 'POST') {
				fired = true;
			}
		});
		await interaction();
		await page.waitForTimeout(500);
		expect(fired).toBe(false);
	}

	async function expectServerRejectionInline(
		page: import('@playwright/test').Page,
	): Promise<void> {
		const respPromise = page.waitForResponse(
			(r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
			{ timeout: 30_000 },
		);
		await submitFirstWorkspace(page);
		const resp = await respPromise;
		expect(resp.ok()).toBe(false);
		await expect(
			page.getByText(/Failed to create workspace\. Please try a different name\./i),
		).toBeVisible();
	}

	test('rejects an empty name with an inline alert and no POST', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await expectNoCreateRequest(page, async () => {
				await submitFirstWorkspace(page);
			});
			await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
		});
	});

	test('rejects a whitespace-only name with an inline alert', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, '   ');
			await expectNoCreateRequest(page, async () => {
				await submitFirstWorkspace(page);
			});
			await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
		});
	});

	test('rejects a name shorter than 4 characters', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, 'abc');
			await expectServerRejectionInline(page);
		});
	});

	test('rejects a name longer than 255 characters', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, 'a'.repeat(256));
			await expectServerRejectionInline(page);
		});
	});

	test('rejects a name containing disallowed characters', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, 'my!workspace');
			await expectServerRejectionInline(page);
		});
	});

	test('trims leading and trailing whitespace before submitting', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			const padded = '  validname-' + Date.now().toString(36) + '  ';
			const expected = padded.trim();
			await fillWorkspaceName(page, padded);
			const respPromise = page.waitForResponse(
				(r) => r.url().endsWith('/api/workspace') && r.request().method() === 'POST',
				{ timeout: 30_000 },
			);
			const reqPromise = page.waitForRequest(
				(r) => r.url().endsWith('/api/workspace') && r.method() === 'POST',
				{ timeout: 30_000 },
			);
			await submitFirstWorkspace(page);
			const [req, resp] = await Promise.all([reqPromise, respPromise]);
			expect(resp.ok()).toBe(true);
			const body = JSON.parse(req.postData() ?? '{}') as { name: string };
			expect(body.name).toBe(expected);
		});
	});

	test('rejects a name already taken by another workspace (CITEXT global unique)', async ({
		browser,
		api,
	}) => {
		const shared = `shared-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
		await using userA = await createUserAccount(api);
		await api.request('POST', '/workspace', {
			token: userA.accessToken,
			clientIp: userA.clientIp,
			body: { name: shared },
		});
		await using userB = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, userB, async (page) => {
			await fillWorkspaceName(page, shared);
			await expectServerRejectionInline(page);
		});
	});

	test('rejects a duplicate name with different casing', async ({ browser, api }) => {
		const base = `case-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
		await using userA = await createUserAccount(api);
		await api.request('POST', '/workspace', {
			token: userA.accessToken,
			clientIp: userA.clientIp,
			body: { name: base.toLowerCase() },
		});
		await using userB = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, userB, async (page) => {
			await fillWorkspaceName(page, base.toUpperCase());
			await expectServerRejectionInline(page);
		});
	});

	test('rejects a unicode-only name', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, '工作空间aaaa');
			await expectServerRejectionInline(page);
		});
	});

	test('rejects an injection-shaped name and keeps the page functional', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, `x'); DROP TABLE workspace;--`);
			await expectServerRejectionInline(page);
			await page.locator('#workspace-name').fill('');
			await fillWorkspaceName(page, 'abcd');
			await expect(page.locator('#workspace-name')).toHaveValue('abcd');
		});
	});
});

test.describe('workspace setup > concurrency & UX @racy', () => {
	test('fires exactly one POST on a rapid double-submit', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await fillWorkspaceName(page, VALID());
			let postCount = 0;
			// Hold the create POST so its success doesn't swap the screen out mid
			// test: the create screen stays mounted, so the suppressed second click
			// still has a button and teardown isn't racing the dashboard swap.
			// fallback() keeps the context's x-real-ip route.
			await page.route('**/api/workspace', async (route) => {
				if (route.request().method() === 'POST') {
					postCount += 1;
					await new Promise((r) => setTimeout(r, 2000));
				}
				await route.fallback();
			});

			// First submit fires POST #1 (held) and disables the button via isLoading.
			await submitFirstWorkspace(page);
			// The second submit is suppressed by the isLoading guard; force the click
			// so Playwright doesn't auto-wait for the disabled button.
			await firstWorkspaceButton(page)
				.click({ force: true })
				.catch(() => undefined);

			// Give a stray second POST a chance to surface, then assert exactly one
			// fired — the rapid double-submit was debounced. (The POST is still held,
			// so the create screen is still mounted and teardown is clean.)
			await page.waitForTimeout(700);
			expect(postCount).toBe(1);
		});
	});

	test('clears the inline error on the next keystroke', async ({ browser, api }) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			await submitFirstWorkspace(page);
			await expect(page.getByText(/Workspace name is required\./i)).toBeVisible();
			await page.locator('#workspace-name').fill('a');
			await expect(page.getByText(/Workspace name is required\./i)).toBeHidden();
		});
	});

	test('renders the create screen inside the app shell (sidebar + topbar)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		await onFirstWorkspaceScreen(browser, user, async (page) => {
			// Unlike the old standalone /onboard page, the create screen now renders
			// within the dashboard shell: the sidebar nav and the workspace switcher
			// are present alongside the form.
			await expectFirstWorkspaceScreen(page);
			await expect(page.getByRole('link', { name: /^Deployments$/ })).toBeVisible();
			await expect(page.getByText('Select A Workspace', { exact: true })).toBeVisible();
		});
	});
});
