import { test, expect, newContext, createUserWithWorkspace, loginAs, expectUrl } from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { createContainerRepo } from '@/helpers/registry';
import { seedMachineType } from '@/helpers/db';
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

test.beforeAll(async () => {
	await seedMachineType();
});

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

	test('the deployments tab pages through the runner deployments', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		for (let i = 0; i < 21; i++) {
			await createDeploymentAPI(api, user, user.workspaceId, {
				repositoryId: repo.id,
				runnerId: runner.id,
			});
		}
		// The tab pages 20 at a time, so page 1 holds exactly one.
		const secondPage = await api.request<{ deployments: { name: string }[] }>(
			'GET',
			`/workspace/${user.workspaceId}/deployment?search[runner]=${runner.id}&page=1&count=20`,
			{ token: user.accessToken, clientIp: user.clientIp },
		);
		expect(secondPage.deployments).toHaveLength(1);
		const last = secondPage.deployments[0].name;

		await withDetail(browser, user, runner.id, undefined, async (page) => {
			await expect(page.getByRole('button', { name: 'Page 2', exact: true })).toBeVisible({
				timeout: 10_000,
			});
			await expect(page.getByText(last, { exact: true })).toHaveCount(0);
			await page.getByRole('button', { name: 'Page 2', exact: true }).click();
			await expectUrl(page, /[?&]page=1(&|$)/);
			await expect(page.getByText(last, { exact: true }).last()).toBeVisible({
				timeout: 10_000,
			});
		});
	});
});
