import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	loginAs,
	RunnerHandle,
} from '@/prelude';
import type { DockerVersion } from '@/prelude';
import { isRunnerConnected } from '@/helpers/runner';
import { waitFor } from '@/helpers/process';
import { listRunnersAPI } from '@/helpers/runner-api';
import {
	openRunnerDetail,
	statusBadge,
	infoTab,
	runnerVersionInput,
	neverConnectedCallout,
} from '@/helpers/ui/runner';

function dv(testInfo: { project: { metadata: Record<string, unknown> } }): DockerVersion {
	return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

// A runner's websocket health is verified by observing that it connects and
// stays stable (the dashboard shows it Online and it keeps reporting connected)
// — not by speaking the raw stream protocol.
test.describe('@docker runner connection lifecycle', () => {
	test('a real runner connects and stays stable (Online in the dashboard)', async ({
		api,
		browser,
	}, testInfo) => {
		test.setTimeout(120_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dv(testInfo),
		});

		// The detail UI shows the Connected chip.
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerDetail(page, runner.runnerId, undefined);
			await expect(statusBadge(page, 'Connected')).toBeVisible({ timeout: 15_000 });

			// Stable: still connected after a settle period well past the debug lock
			// TTL (5s) — i.e. the renewal loop is holding the connection, not a stale
			// lock about to expire.
			await new Promise((r) => setTimeout(r, 8000));
			expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(
				true,
			);

			const list = await listRunnersAPI(api, user, user.workspaceId, '?page=0&count=100');
			const row = list.runners.find((r) => r.id === runner.runnerId);
			expect(row?.connected).toBe(true);
			expect(row?.lastSeen).not.toBeNull();

			// Info shows the version it reported, current with the API, and no setup callout.
			await infoTab(page).click();
			await expect(runnerVersionInput(page)).toHaveValue(row!.version, { timeout: 10_000 });
			await expect(neverConnectedCallout(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('graceful shutdown releases the lock → runner flips to disconnected', async ({
		api,
	}, testInfo) => {
		test.setTimeout(120_000);
		await using user = await createUserWithWorkspace(api);
		const runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dv(testInfo),
		});
		let disposed = false;
		try {
			expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(
				true,
			);

			// Graceful SIGTERM: the runner releases its connection lock on shutdown.
			await runner[Symbol.asyncDispose]();
			disposed = true;

			// connected (lock-derived) flips to false promptly.
			let connected = true;
			for (let i = 0; i < 15; i++) {
				connected = await isRunnerConnected(api, user, user.workspaceId, runner.runnerId);
				if (!connected) break;
				await new Promise((r) => setTimeout(r, 1000));
			}
			expect(connected).toBe(false);
		} finally {
			if (!disposed) await runner[Symbol.asyncDispose]();
		}
	});

	test('a connected runner ships its system metrics to Mimir', async ({ api }, testInfo) => {
		test.setTimeout(300_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dv(testInfo),
		});

		// The runner's Alloy service scrapes the host every minute and remote-writes
		// through the API's mimir proxy with the runner token. The workspace is left
		// unconfigured in the observability stub, so this read reaches the real Mimir.
		let values: string[] = [];
		await waitFor(
			async () => {
				const res = await api.request<{ dataPoints: Array<{ value: string }> }>(
					'GET',
					`/workspace/${user.workspaceId}/runner/${runner.runnerId}/metrics/system_memory_usage`,
					{ token: user.accessToken, clientIp: user.clientIp },
				);
				values = res.dataPoints.map((d) => d.value);
				return values.length > 0;
			},
			{ timeoutMs: 180_000, intervalMs: 5000, label: 'runner memory usage in mimir' },
		);
		// Memory usage is a percentage of the DinD host's memory.
		const usage = Number(values.at(-1));
		expect(usage).toBeGreaterThan(0);
		expect(usage).toBeLessThanOrEqual(100);
	});
});
