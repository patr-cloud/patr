import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';
import { randomRunnerName } from '@/helpers/runner-api';
import {
	openRunnerCreate,
	fillRunnerName,
	submitCreateRunner,
	nameErrorAlert,
	openRunnerList,
	runnerRow,
} from '@/helpers/ui/runner';

// Runner creation is exercised through the dashboard. The create form only
// blocks empty/whitespace client-side; every other invalid name is POSTed and
// the server failure surfaces as a generic inline alert. Exact status codes,
// duplicate→409/reusable-after-delete and cross-workspace uniqueness live in the
// Rust API suite (api/tests/api/workspace/runner.rs).

async function withCreatePage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openRunnerCreate(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

function trackCreatePosts(page: import('@playwright/test').Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'POST' && /\/api\/workspace\/[^/]+\/runner$/.test(req.url())) {
			count += 1;
		}
	});
	return () => count;
}

test.describe('runner > create [UI]', () => {
	test('creates a runner: success toast, navigate to /runners, row visible', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await withCreatePage(browser, user, async (page) => {
			await fillRunnerName(page, name);
			await submitCreateRunner(page);
			await expectToast(page, /Runner created successfully/i);
			await expectUrl(page, /\/runners$/, { timeout: 10_000 });
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('accepts an uppercase / space / dot name', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = `My Runner.${crypto.randomUUID().slice(0, 6)}`;
		await withCreatePage(browser, user, async (page) => {
			await fillRunnerName(page, name);
			await submitCreateRunner(page);
			await expectToast(page, /Runner created successfully/i);
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('trims surrounding whitespace from the stored name', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomRunnerName();
		await withCreatePage(browser, user, async (page) => {
			await fillRunnerName(page, `  ${name}  `);
			await submitCreateRunner(page);
			await expectUrl(page, /\/runners$/, { timeout: 10_000 });
			// The row shows the trimmed name.
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('empty name: inline error and no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackCreatePosts(page);
			await submitCreateRunner(page);
			await expect(nameErrorAlert(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('whitespace-only name: blocked client-side, no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackCreatePosts(page);
			await fillRunnerName(page, '   ');
			await submitCreateRunner(page);
			await expect(nameErrorAlert(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(posts()).toBe(0);
		});
	});

	test('a server-rejected invalid name shows a generic failure alert, no navigation', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			// Non-empty but regex-invalid → passes the client guard, POSTs, 400s.
			await fillRunnerName(page, 'ab/cd');
			await submitCreateRunner(page);
			await expect(page.getByText(/Failed to create runner/i)).toBeVisible({
				timeout: 10_000,
			});
			await expectUrl(page, /\/runners\/new$/, { timeout: 3_000 });
		});
	});
});
