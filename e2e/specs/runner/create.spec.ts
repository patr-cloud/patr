import { test, expect, newContext, createUserWithWorkspace, loginAs, expectUrl } from '@/prelude';
import { createRunnerAPI, randomRunnerName } from '@/helpers/runner-api';
import {
	openRunnerCreate,
	openRunnerList,
	emptyStateHeading,
	addRunnerLink,
	runnerRow,
	runnerNameInput,
	fillRunnerName,
	submitCreateRunner,
	nameRequiredError,
	nameCharactersError,
	nameTakenError,
	setupCommand,
	tokenShownOnceAlert,
	runnerTokenField,
	copyRunnerTokenButton,
	reconnectCommand,
	goToRunnerButton,
	runnersBreadcrumb,
	leaveWithoutTokenModal,
	statusBadge,
} from '@/helpers/ui/runner';

// The dashboard creates the runner, then shows its token once, inside the
// `runner setup reconnect` command. Name rules beyond the client-side checks
// (reusable after delete, cross-workspace uniqueness) live in the Rust API
// suite — api/tests/api/workspace/runner.rs.

type Page = import('@playwright/test').Page;

async function withPage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
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

function trackCreatePosts(page: Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'POST' && /\/api\/workspace\/[^/]+\/runner$/.test(req.url())) {
			count += 1;
		}
	});
	return () => count;
}

// Submits the form and reads the id and token from the CreateRunner response,
// so the spec can check what the page shows against what the API returned.
async function createFromForm(page: Page, name: string): Promise<{ id: string; token: string }> {
	const response = page.waitForResponse(
		(r) => r.request().method() === 'POST' && /\/api\/workspace\/[^/]+\/runner$/.test(r.url()),
	);
	await fillRunnerName(page, name);
	await submitCreateRunner(page);
	const resp = await response;
	expect(resp.status()).toBe(201);
	return (await resp.json()) as { id: string; token: string };
}

test.describe('runner > create [UI]', () => {
	test('shows the token once, on its own and in the reconnect command', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const runner = await createFromForm(page, randomRunnerName());
			expect(runner.token).toMatch(/^patr_sa_/);

			await expect(tokenShownOnceAlert(page)).toBeVisible({ timeout: 10_000 });
			await expect(
				reconnectCommand(page, user.workspaceId, runner.id, runner.token),
			).toBeVisible();
			await expect(runnerTokenField(page, runner.token)).toBeVisible();
			await expect(
				setupCommand(
					page,
					'curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/develop/assets/cli/install.sh | sh',
				),
			).toBeVisible();
			await expect(setupCommand(page, 'patr runner service install')).toBeVisible();
			await expect(page.getByText(/Linux with systemd/)).toBeVisible();
			await expect(runnerNameInput(page)).toHaveCount(0);
		});
	});

	test('Go to Runner opens the new runner without asking', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const runner = await createFromForm(page, randomRunnerName());
			await goToRunnerButton(page).click();
			await expectUrl(page, new RegExp(`/runners/${runner.id}(\\?|$)`), { timeout: 10_000 });
			await expect(statusBadge(page, 'Not set up')).toBeVisible({ timeout: 10_000 });
			await expect(leaveWithoutTokenModal(page)).toHaveCount(0);
		});
	});

	test('the new runner shows up in the list', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await withPage(browser, user, async (page) => {
			// Start on the list, so it's already loaded when we come back to it.
			await openRunnerList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 10_000 });
			await addRunnerLink(page).first().click();
			await expect(runnerNameInput(page)).toBeVisible({ timeout: 10_000 });
			await createFromForm(page, name);
			await goToRunnerButton(page).click();
			await runnersBreadcrumb(page).click();
			await expectUrl(page, /\/runners$/, { timeout: 10_000 });
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	// Navigation-blocking: tagged @racy so it runs in the serial pass.
	test('@racy leaving asks first, and the token is gone on the way back', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const runner = await createFromForm(page, name);
			const command = reconnectCommand(page, user.workspaceId, runner.id, runner.token);
			await expect(command).toBeVisible({ timeout: 10_000 });

			await runnersBreadcrumb(page).click();
			await expect(leaveWithoutTokenModal(page)).toBeVisible({ timeout: 5_000 });
			await page.getByRole('button', { name: /^Stay$/ }).click();
			await expect(command).toBeVisible();
			await expectUrl(page, /\/runners\/new$/, { timeout: 3_000 });

			await runnersBreadcrumb(page).click();
			await expect(leaveWithoutTokenModal(page)).toBeVisible({ timeout: 5_000 });
			await page.getByRole('button', { name: /^Leave$/ }).click();
			await expectUrl(page, /\/runners$/, { timeout: 10_000 });
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });

			await addRunnerLink(page).first().click();
			await expect(runnerNameInput(page)).toBeVisible({ timeout: 10_000 });
			await expect(page.getByText(runner.token)).toHaveCount(0);
			await expect(tokenShownOnceAlert(page)).toHaveCount(0);
		});
	});

	// Navigation-blocking: tagged @racy so it runs in the serial pass.
	test("@racy once the token is copied, leaving doesn't ask", async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await withPage(browser, user, async (page) => {
			await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
			await openRunnerCreate(page);
			const runner = await createFromForm(page, name);

			await copyRunnerTokenButton(page, runner.token).click();
			expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(runner.token);

			await runnersBreadcrumb(page).click();
			await expectUrl(page, /\/runners$/, { timeout: 10_000 });
			await expect(leaveWithoutTokenModal(page)).toHaveCount(0);
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('the token never goes in a URL', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			const urls: string[] = [];
			page.on('framenavigated', (frame) => urls.push(frame.url()));
			page.on('request', (req) => urls.push(req.url()));

			await openRunnerCreate(page);
			const runner = await createFromForm(page, randomRunnerName());
			await expect(tokenShownOnceAlert(page)).toBeVisible({ timeout: 10_000 });
			expect(page.url()).not.toContain(runner.token);

			await goToRunnerButton(page).click();
			await expectUrl(page, new RegExp(`/runners/${runner.id}(\\?|$)`), { timeout: 10_000 });
			expect(urls.filter((url) => url.includes(runner.token))).toEqual([]);
		});
	});

	test('empty name: inline error and no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const posts = trackCreatePosts(page);
			await submitCreateRunner(page);
			await expect(nameRequiredError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('whitespace-only name: inline error and no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const posts = trackCreatePosts(page);
			await fillRunnerName(page, '   ');
			await submitCreateRunner(page);
			await expect(nameRequiredError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('invalid characters: inline error naming the allowed ones, no network call', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			const posts = trackCreatePosts(page);
			await fillRunnerName(page, 'ab/cd');
			await submitCreateRunner(page);
			await expect(nameCharactersError(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('a duplicate name shows an inline error and no token', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await createRunnerAPI(api, user, user.workspaceId, name);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			await fillRunnerName(page, name);
			await submitCreateRunner(page);
			await expect(nameTakenError(page, name)).toBeVisible({ timeout: 10_000 });
			await expect(tokenShownOnceAlert(page)).toHaveCount(0);
			await expectUrl(page, /\/runners\/new$/, { timeout: 3_000 });
		});
	});

	test('notes the CLI-only path with the current workspace', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerCreate(page);
			await expect(setupCommand(page, 'patr login')).toBeVisible();
			await expect(
				setupCommand(page, `patr -w ${user.workspaceId} runner setup new`),
			).toBeVisible();
		});
	});
});
