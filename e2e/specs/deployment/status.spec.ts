import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType, sql } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, getDeploymentInfoAPI } from '@/helpers/deployment-api';
import {
	openDeploymentList,
	openDeploymentDetail,
	deploymentRowStatus,
	detailStatus,
} from '@/helpers/ui/deployment';
import { openRunnerDetail, statusBadge } from '@/helpers/ui/runner';

// How the API derives a status from `should_run`, `observed_status` and the
// runner being online lives in the Rust API suite. Here we cover the labels the
// dashboard shows for each one.

test.beforeAll(async () => {
	await seedMachineType();
});

// The `should_run` and `observed_status` that derive each status on an online runner.
const STATUSES: { label: string; shouldRun: boolean; observed: string | null }[] = [
	{ label: 'Running', shouldRun: true, observed: 'running' },
	{ label: 'Deploying', shouldRun: true, observed: null },
	{ label: 'Crashing', shouldRun: true, observed: 'crashing' },
	{ label: 'Health check failing', shouldRun: true, observed: 'health_check_failing' },
	{ label: 'Image pull failed', shouldRun: true, observed: 'image_pull_failed' },
	{ label: 'Invalid architecture', shouldRun: true, observed: 'invalid_architecture' },
	{ label: 'Errored', shouldRun: true, observed: 'errored' },
	{ label: 'Stopped', shouldRun: false, observed: null },
	{ label: 'Stopping', shouldRun: false, observed: 'running' },
];

test.describe('deployment > status [UI]', () => {
	// The suite has no connected runner by default, so every deployment it
	// creates is on an offline runner and reads `unreachable`.
	test('a deployment on a never-connected runner reads Runner offline', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
		});
		expect((await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id)).status).toBe(
			'unreachable',
		);

		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(deploymentRowStatus(page, dep.name, 'Runner offline')).toBeVisible({
				timeout: 15_000,
			});

			await openDeploymentDetail(page, dep.id);
			await expect(detailStatus(page, 'Runner offline')).toBeVisible({ timeout: 15_000 });

			// The runner keeps its own status, "Not set up" since it never connected; its
			// deployments read "Runner offline".
			await openRunnerDetail(page, runner.id);
			await expect(statusBadge(page, 'Not set up')).toBeVisible({ timeout: 15_000 });
			await expect(deploymentRowStatus(page, dep.name, 'Runner offline')).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test('each status shows its label in the deployment list', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const online = await createRunnerAPI(api, user, user.workspaceId);
		const offline = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		// Only a runner's websocket marks it online, and the suite has none
		// connected, so mark one online in the DB, as its last report would.
		await sql(`UPDATE runner SET is_connected = TRUE, last_seen = NOW() WHERE id = $1`, [
			online.id,
		]);

		const rows: { name: string; label: string }[] = [];
		for (const status of STATUSES) {
			const dep = await createDeploymentAPI(api, user, user.workspaceId, {
				repositoryId: repo.id,
				runnerId: online.id,
			});
			await sql(`UPDATE deployment SET should_run = $2, observed_status = $3 WHERE id = $1`, [
				dep.id,
				status.shouldRun,
				status.observed,
			]);
			rows.push({ name: dep.name, label: status.label });
		}
		const unreachable = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: offline.id,
		});
		rows.push({ name: unreachable.name, label: 'Runner offline' });

		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			for (const row of rows) {
				await expect(deploymentRowStatus(page, row.name, row.label)).toBeVisible({
					timeout: 15_000,
				});
			}
		} finally {
			await context.close();
		}
	});
});
