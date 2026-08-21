import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	loginAs,
	expectUrl,
	addMemberToWorkspace,
	listRolesAPI,
} from '@/prelude';
import {
	openWorkspaceSettings,
	setWorkspaceName,
	clickUpdate,
	expectUpdateDisabled,
	expectUpdateEnabled,
	expectToast,
	expectFirstWorkspaceScreen,
	getLastWorkspaceIdCookie,
} from '@/helpers/ui/workspace';

const VALID = () => `wks-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

async function withSettings(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (
		page: import('@playwright/test').Page,
		context: import('@playwright/test').BrowserContext,
	) => Promise<void>,
) {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openWorkspaceSettings(page);
		await fn(page, context);
	} finally {
		await context.close();
	}
}

// Wait for the GET /workspace/{id} response so the form is populated before
// asserting on the prefilled value.
async function waitForWorkspaceInfo(page: import('@playwright/test').Page): Promise<void> {
	await expect(page.locator('#workspace-name')).not.toHaveValue('', {
		timeout: 10_000,
	});
}

test.describe('workspace settings > route guards', () => {
	test('redirects unauthenticated visits to /workspace to /login', async ({ browser }) => {
		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expectUrl(page, /\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});

	test('shows the create-workspace screen for a zero-workspace user at /workspace', async ({
		browser,
		api,
	}) => {
		await using user = await createUserAccount(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user);
		const page = await context.newPage();
		try {
			// /workspace is under _workspaced; with no workspace the layout renders
			// the inline create-first-workspace screen in place of the settings page.
			await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
			await expectFirstWorkspaceScreen(page);
		} finally {
			await context.close();
		}
	});
});

test.describe('workspace settings > field state', () => {
	test('renders the workspace id matching the active lastWorkspaceId cookie', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page, context) => {
			const cookieId = await getLastWorkspaceIdCookie(context);
			expect(cookieId).toBe(user.workspaceId);
			await expect(page.getByText(user.workspaceId).first()).toBeVisible({
				timeout: 10_000,
			});
		});
	});

	test('pre-fills the workspace-name input with the current name', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await expect(page.locator('#workspace-name')).toHaveValue(user.workspaceName);
		});
	});

	test('disables Update when the name is unchanged', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await expectUpdateDisabled(page);
		});
	});

	test('disables Update when the name is cleared', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await page.locator('#workspace-name').fill('');
			await expectUpdateDisabled(page);
		});
	});

	test('disables Update when the name is whitespace only', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await page.locator('#workspace-name').fill('   ');
			await expectUpdateDisabled(page);
		});
	});

	test('enables Update when the name changes to a non-empty differing value', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, VALID());
			await expectUpdateEnabled(page);
		});
	});
});

test.describe('workspace settings > rename', () => {
	test('renames the workspace, shows a success toast, and persists in the input', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			const newName = VALID();
			await setWorkspaceName(page, newName);
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await clickUpdate(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(true);
			await expectToast(page, /Workspace name updated successfully/i);
			await expect(page.locator('#workspace-name')).toHaveValue(newName);
		});
	});

	test('updates the sidebar switcher entry after rename without a page reload', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			const newName = VALID();
			await setWorkspaceName(page, newName);
			await clickUpdate(page);
			await expectToast(page, /Workspace name updated successfully/i);
			await expect(page.getByText(newName).first()).toBeVisible({
				timeout: 10_000,
			});
		});
	});

	test('rejects a rename to a globally-taken name with an error toast', async ({
		browser,
		api,
	}) => {
		const taken = `taken-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
		await using userA = await createUserAccount(api);
		await api.request('POST', '/workspace', {
			token: userA.accessToken,
			clientIp: userA.clientIp,
			body: { name: taken },
		});
		await using userB = await createUserWithWorkspace(api);
		await withSettings(browser, userB, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, taken);
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${userB.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await clickUpdate(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
			await expectToast(page, /Failed to update workspace name/i);
		});
	});

	test('rejects a rename shorter than 4 characters', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, 'abc');
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await clickUpdate(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
			await expectToast(page, /Failed to update workspace name/i);
		});
	});

	test('rejects a rename longer than 255 characters', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, 'a'.repeat(256));
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await clickUpdate(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
			await expectToast(page, /Failed to update workspace name/i);
		});
	});

	test('rejects a rename with disallowed characters', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, 'bad@name');
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await clickUpdate(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
			await expectToast(page, /Failed to update workspace name/i);
		});
	});

	test('trims leading and trailing whitespace before the PATCH', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			const padded = '  rename-' + Date.now().toString(36) + '  ';
			const expected = padded.trim();
			const reqPromise = page.waitForRequest(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await setWorkspaceName(page, padded);
			await clickUpdate(page);
			const req = await reqPromise;
			const body = JSON.parse(req.postData() ?? '{}') as { name: string };
			expect(body.name).toBe(expected);
		});
	});

	// PATCH-without-editWorkspace gating is covered in the Rust API suite
	// (api/tests/api/workspace/mod.rs::update_workspace_denied_without_edit_permission).

	test('fires exactly one PATCH on a rapid double-click of Update', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withSettings(browser, user, async (page) => {
			await waitForWorkspaceInfo(page);
			await setWorkspaceName(page, VALID());
			let count = 0;
			page.on('request', (req) => {
				if (
					req.url().includes(`/api/workspace/${user.workspaceId}`) &&
					req.method() === 'PATCH'
				) {
					count++;
				}
			});
			const respPromise = page.waitForResponse(
				(r) =>
					r.url().includes(`/api/workspace/${user.workspaceId}`) &&
					r.request().method() === 'PATCH',
				{ timeout: 30_000 },
			);
			await Promise.all([clickUpdate(page), clickUpdate(page).catch(() => undefined)]);
			await respPromise;
			await page.waitForTimeout(500);
			expect(count).toBe(1);
		});
	});
});
