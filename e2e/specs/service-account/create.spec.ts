import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	loginAs,
	expectUrl,
	callWithApiToken,
} from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	builtInRoleId,
	createServiceAccountAPI,
	getServiceAccountAPI,
	randomServiceAccountName,
} from '@/helpers/service-account-api';
import {
	openServiceAccountCreate,
	openServiceAccountList,
	nameInput,
	descriptionInput,
	selectBindingRole,
	removeBindingRole,
	submitCreateServiceAccount,
	nameRequiredError,
	nameCharactersError,
	nameTakenError,
	descriptionCharactersError,
	noRolesError,
	tokenShownOnceAlert,
	copyableValue,
	copyTokenButton,
	goToServiceAccountButton,
	serviceAccountsBreadcrumb,
	leaveWithoutTokenModal,
	serviceAccountRow,
	createServiceAccountLink,
} from '@/helpers/ui/service-account';

// The dashboard creates the service account, then shows its token once on the
// same page. Name and binding rules at the API layer live in the Rust API
// suite — api/tests/api/workspace/service_account.rs.

type Page = import('@playwright/test').Page;
type Owner = Awaited<ReturnType<typeof createUserWithWorkspace>>;

async function withPage(
	browser: import('@playwright/test').Browser,
	user: Owner,
	fn: (page: Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

const CREATE_URL = /\/api\/workspace\/[^/]+\/service-account$/;

function trackCreatePosts(page: Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'POST' && CREATE_URL.test(req.url())) {
			count += 1;
		}
	});
	return () => count;
}

// Fills the form, submits it and reads the id and token from the response, so
// the spec can check what the page shows against what the API returned.
async function createFromForm(
	page: Page,
	opts: { name: string; description?: string; role: string },
): Promise<{ id: string; token: string }> {
	await nameInput(page).fill(opts.name);
	if (opts.description !== undefined) {
		await descriptionInput(page).fill(opts.description);
	}
	await selectBindingRole(page, opts.role);
	const response = page.waitForResponse(
		(r) => r.request().method() === 'POST' && CREATE_URL.test(r.url()),
	);
	await submitCreateServiceAccount(page);
	const resp = await response;
	expect(resp.status()).toBe(201);
	return (await resp.json()) as { id: string; token: string };
}

test.describe('service account > create [UI]', () => {
	test('shows the token once, and how to use it', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		let accountId = '';
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, {
				name: randomServiceAccountName(),
				description: 'Deploys from GitHub Actions',
				role: 'Runner: Viewer',
			});
			accountId = account.id;
			expect(account.token).toMatch(/^patr_sa_/);

			await expect(tokenShownOnceAlert(page)).toBeVisible({ timeout: 10_000 });
			await expect(copyableValue(page, account.token)).toBeVisible();
			await expect(
				copyableValue(page, 'docker login registry.patr.cloud -u patr'),
			).toBeVisible();
			await expect(nameInput(page)).toHaveCount(0);
			// The CLI can't use these tokens, so the page never suggests it.
			await expect(page.getByText(/PATR_TOKEN|patr login|patr -w/)).toHaveCount(0);
			expect(page.url()).not.toContain(account.token);
		});

		const info = await getServiceAccountAPI(api, owner, owner.workspaceId, accountId);
		expect(info.description).toBe('Deploys from GitHub Actions');
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		expect(info.roleBindings).toEqual([
			{ roleId: runnerViewer, resourceId: owner.workspaceId },
		]);
	});

	test('the token works as a bearer token for the API', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, {
				name: randomServiceAccountName(),
				role: 'Runner: Viewer',
			});
			const list = await callWithApiToken(api, account.token, {
				clientIp: owner.clientIp,
				path: `/workspace/${owner.workspaceId}/runner`,
			});
			expect(list.status).toBe(200);
			expect((list.body as { runners: { id: string }[] }).runners.map((r) => r.id)).toContain(
				runner.id,
			);
		});
	});

	test('a blank description is saved as none', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomServiceAccountName();
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, {
				name,
				description: '   ',
				role: 'Runner: Viewer',
			});
			const info = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
			expect(info.description).toBeNull();
		});
	});

	test('Go to Service Account opens it without asking', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomServiceAccountName();
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, { name, role: 'Runner: Viewer' });
			await goToServiceAccountButton(page).click();
			await expectUrl(page, new RegExp(`/workspace/service-accounts/${account.id}$`), {
				timeout: 10_000,
			});
			await expect(nameInput(page)).toHaveValue(name, { timeout: 10_000 });
			await expect(leaveWithoutTokenModal(page)).toHaveCount(0);
		});
	});

	test('the token is gone after going back', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomServiceAccountName();
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, { name, role: 'Runner: Viewer' });
			await expect(copyableValue(page, account.token)).toBeVisible({ timeout: 10_000 });
			await goToServiceAccountButton(page).click();
			await expectUrl(page, new RegExp(`/workspace/service-accounts/${account.id}$`), {
				timeout: 10_000,
			});
			// The URL changes before the page does, so wait for the account's page to render.
			await expect(nameInput(page)).toHaveValue(name, { timeout: 10_000 });

			await page.goBack();
			await expectUrl(page, /\/workspace\/service-accounts\/new$/, { timeout: 10_000 });
			await expect(nameInput(page)).toBeVisible({ timeout: 10_000 });
			await expect(page.getByText(account.token)).toHaveCount(0);
			await expect(tokenShownOnceAlert(page)).toHaveCount(0);
		});
	});

	// Navigation-blocking: tagged @racy so it runs in the serial pass.
	test('@racy leaving asks first, until the token is copied', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomServiceAccountName();
		await withPage(browser, owner, async (page) => {
			await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
			await openServiceAccountCreate(page);
			const account = await createFromForm(page, { name, role: 'Runner: Viewer' });
			await expect(copyableValue(page, account.token)).toBeVisible({ timeout: 10_000 });

			await serviceAccountsBreadcrumb(page).click();
			await expect(leaveWithoutTokenModal(page)).toBeVisible({ timeout: 5_000 });
			await page.getByRole('button', { name: /^Stay$/ }).click();
			await expect(copyableValue(page, account.token)).toBeVisible();
			await expectUrl(page, /\/workspace\/service-accounts\/new$/, { timeout: 3_000 });

			await copyTokenButton(page, account.token).click();
			expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(account.token);

			await serviceAccountsBreadcrumb(page).click();
			await expectUrl(page, /\/workspace\/service-accounts$/, { timeout: 10_000 });
			await expect(leaveWithoutTokenModal(page)).toHaveCount(0);
			await expect(serviceAccountRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('empty name: inline error and no network call', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const posts = trackCreatePosts(page);
			await selectBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await expect(nameRequiredError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('invalid characters: inline error naming the allowed ones', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const posts = trackCreatePosts(page);
			await nameInput(page).fill('ci/deploy');
			await selectBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await expect(nameCharactersError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('a description with markup is caught inline', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const posts = trackCreatePosts(page);
			await nameInput(page).fill(randomServiceAccountName());
			await descriptionInput(page).fill('<b>deploys</b>');
			await expect(descriptionCharactersError(page)).toBeVisible();
			await selectBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('the description is capped at 500 characters', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			await descriptionInput(page).fill('a'.repeat(600));
			await expect(descriptionInput(page)).toHaveValue('a'.repeat(500));
		});
	});

	test('at least one role is required', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			const posts = trackCreatePosts(page);
			await nameInput(page).fill(randomServiceAccountName());
			await selectBindingRole(page, 'Runner: Viewer');
			await removeBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await expect(noRolesError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('a duplicate name shows an inline error and no token', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const existing = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountCreate(page);
			await nameInput(page).fill(existing.name);
			await selectBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await expect(nameTakenError(page, existing.name)).toBeVisible({ timeout: 10_000 });
			await expect(tokenShownOnceAlert(page)).toHaveCount(0);
			await expectUrl(page, /\/workspace\/service-accounts\/new$/, { timeout: 3_000 });
		});
	});

	test('the header button opens the form', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		await createServiceAccountAPI(api, owner, owner.workspaceId, { roles: [roleId] });
		await withPage(browser, owner, async (page) => {
			await openServiceAccountList(page);
			await createServiceAccountLink(page).click();
			await expect(nameInput(page)).toBeVisible({ timeout: 10_000 });
		});
	});
});
