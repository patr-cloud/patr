import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	openRunnerDetail,
	statusBadge,
	deploymentsTab,
	infoTab,
	metricsTab,
	logsTab,
	deleteRunnerButton,
	neverConnectedCallout,
	runnerVersionInput,
	runnerDeploymentsEmptyState,
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
	test('detail shows the four tabs and a Not set up badge for a fresh runner', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, undefined, async (page) => {
			await expect(deploymentsTab(page)).toBeVisible();
			await expect(infoTab(page)).toBeVisible();
			await expect(metricsTab(page)).toBeVisible();
			await expect(logsTab(page)).toBeVisible();
			await expect(statusBadge(page, 'Not set up')).toBeVisible();
		});
	});

	test('a never-connected runner points at the reconnect command', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, undefined, async (page) => {
			// The callout lives in the Info tab, not over the default Deployments tab.
			await expect(runnerDeploymentsEmptyState(page)).toBeVisible({ timeout: 10_000 });
			await expect(neverConnectedCallout(page)).toHaveCount(0);

			await infoTab(page).click();
			await expect(neverConnectedCallout(page)).toBeVisible({ timeout: 10_000 });
			await expect(
				page.getByText(
					`patr -w ${user.workspaceId} runner setup reconnect --runner-id ${runner.id}`,
					{ exact: true },
				),
			).toBeVisible();
			// The owner can regenerate, so the callout says where the token comes from.
			await expect(page.getByText(/use Regenerate Token above/)).toBeVisible();
			// It hasn't reported a version, and there's nothing to upgrade.
			await expect(runnerVersionInput(page)).toHaveValue('Unknown');
			await expect(page.getByText(/patr upgrade/)).toHaveCount(0);
		});
		await withDetail(browser, user, runner.id, 'metrics', async (page) => {
			await expect(page.getByText('Interval', { exact: true })).toBeVisible({
				timeout: 10_000,
			});
			await expect(neverConnectedCallout(page)).toHaveCount(0);
		});
	});

	test('the workspace owner sees a delete control on the runner detail', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withDetail(browser, user, runner.id, undefined, async (page) => {
			await expect(deploymentsTab(page)).toBeVisible();
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
