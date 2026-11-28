import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType, sql } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, getDeploymentInfoAPI } from '@/helpers/deployment-api';
import {
	openDeploymentDetail,
	startButton,
	stopButton,
	detailStatus,
} from '@/helpers/ui/deployment';

// Start/stop gating and what the runner does with them live in the Rust API
// suite and the @docker execution spec. Here we cover the dashboard's buttons,
// which follow `shouldRun` rather than the status (the suite's runner is never
// connected, so the deployment starts out `unreachable`), and their toasts.

test.beforeAll(async () => {
	await seedMachineType();
});

test.describe('deployment > start/stop [UI]', () => {
	test('Start and Stop flip shouldRun and swap the buttons', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		// Created without deploying, so it starts out meant to be stopped.
		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		// The toasts name the runner once the page has fetched it.
		const runnerFetched = () =>
			page.waitForResponse((r) => r.url().endsWith(`/runner/${runner.id}`) && r.ok());
		try {
			let fetched = runnerFetched();
			await openDeploymentDetail(page, dep.id);
			await fetched;
			await expect(detailStatus(page, 'Runner offline')).toBeVisible({ timeout: 15_000 });
			await expect(startButton(page)).toBeVisible();
			await expect(stopButton(page)).toHaveCount(0);

			await startButton(page).click();
			await expect(
				page.getByText(
					`Start requested. The runner "${runner.name}" is offline, so "${dep.name}" will start once it reconnects.`,
					{ exact: true },
				),
			).toBeVisible();
			await expect(stopButton(page)).toBeVisible({ timeout: 15_000 });
			await expect(startButton(page)).toHaveCount(0);
			expect(
				(await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id)).shouldRun,
			).toBe(true);

			// Mark the runner online, as its websocket would, for the online wording.
			await sql(`UPDATE runner SET is_connected = TRUE, last_seen = NOW() WHERE id = $1`, [
				runner.id,
			]);
			fetched = runnerFetched();
			await openDeploymentDetail(page, dep.id);
			await fetched;
			await stopButton(page).click();
			await expect(
				page.getByText(
					`Stop requested. The runner "${runner.name}" will shut down "${dep.name}". Its volumes are kept.`,
					{ exact: true },
				),
			).toBeVisible();
			await expect(startButton(page)).toBeVisible({ timeout: 15_000 });
			await expect(stopButton(page)).toHaveCount(0);
			expect(
				(await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id)).shouldRun,
			).toBe(false);
		} finally {
			await context.close();
		}
	});
});
