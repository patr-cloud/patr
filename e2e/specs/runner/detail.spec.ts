import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	openRunnerDetail,
	statusBadge,
	infoTab,
	metricsTab,
	logsTab,
	deleteRunnerButton,
} from '@/helpers/ui/runner';

// get-info anti-enum 401, delete→202/inaccessible and delete-already-deleted at
// the API layer live in the Rust API suite (api/tests/api/workspace/runner.rs).
// Here we cover only the dashboard surface.

async function withDetail(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	id: string,
	tab: string | undefined,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openRunnerDetail(page, id, tab);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('runner > detail [UI]', () => {
	test('detail shows the three tabs and an Unreachable badge for a fresh runner', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, undefined, async (page) => {
			await expect(infoTab(page)).toBeVisible();
			await expect(metricsTab(page)).toBeVisible();
			await expect(logsTab(page)).toBeVisible();
			await expect(statusBadge(page, 'Unreachable')).toBeVisible();
		});
	});

	test('the workspace owner sees a delete control on the runner detail', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, undefined, async (page) => {
			await expect(infoTab(page)).toBeVisible();
			await expect(deleteRunnerButton(page)).toBeVisible();
		});
	});

	test('?tab=metrics and ?tab=logs select their tabs', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, 'metrics', async (page) => {
			await expect(metricsTab(page)).toBeVisible();
		});
		await withDetail(browser, user, runner.id, 'logs', async (page) => {
			await expect(logsTab(page)).toBeVisible();
		});
	});
});
