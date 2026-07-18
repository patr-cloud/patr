import {
	test,
	expect,
	newContext,
	loginAs,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import { openRunnerDetail, metricsTab } from '@/helpers/ui/runner';
import {
	configureObservability,
	observabilityRequests,
	resetObservability,
} from '@/helpers/observability';

// Runner logs come from Loki, metrics from Mimir. The e2e stack points both at
// the observability stub (helpers/observability.ts), keyed by x-scope-orgid =
// workspace id, so we can serve deterministic data, assert the LogQL/PromQL and
// the org header, and exercise the parse-error (500) path. No runner process is
// needed — a runner row (API-created) is enough to drive the read path.

const base = (ws: string) => `/workspace/${ws}/runner`;

const METRIC_NAMES = [
	'system_cpu_usage',
	'system_memory_usage',
	'system_disk_read_bytes',
	'system_disk_written_bytes',
	'system_disk_usage',
	'system_network_rx',
	'system_network_tx',
];

test.describe('runner > logs [API]', () => {
	test('serves runner logs and records the LogQL + org header', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, {
			loki: { values: [['1700000000000000000', 'runner boot complete']] },
		});

		const res = await api.request<{ logs: Array<{ log: string }> }>(
			'GET',
			`${base(user.workspaceId)}/${runner.id}/logs`,
			{ token: user.accessToken, clientIp: user.clientIp },
		);
		expect(res.logs.map((l) => l.log)).toContain('runner boot complete');

		const reqs = await observabilityRequests(user.workspaceId);
		const loki = reqs.find((r) => r.kind === 'loki');
		expect(loki?.headers['x-scope-orgid']).toBe(user.workspaceId);
		// LogQL targets this runner's logs.
		expect(loki?.query.query).toContain(runner.id);
		expect(loki?.query.query).toContain('source="runner"');
	});

	test('passes a search filter through to the LogQL query', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, { loki: { values: [] } });
		await api.request('GET', `${base(user.workspaceId)}/${runner.id}/logs?search=panic`, {
			token: user.accessToken,
			clientIp: user.clientIp,
		});
		const reqs = await observabilityRequests(user.workspaceId);
		expect(reqs.find((r) => r.kind === 'loki')?.query.query).toContain('panic');
	});

	test('a malformed Loki response surfaces a 500', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, { malformed: 'loki' });
		await expect(
			api.request('GET', `${base(user.workspaceId)}/${runner.id}/logs`, {
				token: user.accessToken,
				clientIp: user.clientIp,
			}),
		).rejects.toThrow(/500/);
	});
});

test.describe('runner > metrics [API]', () => {
	test('serves each of the 7 metric names with the org header', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		for (const metric of METRIC_NAMES) {
			await resetObservability(user.workspaceId);
			await configureObservability(user.workspaceId, {
				mimir: { values: [[1700000000, '42.5']] },
			});
			const res = await api.request<{ dataPoints: Array<{ value: string }> }>(
				'GET',
				`${base(user.workspaceId)}/${runner.id}/metrics/${metric}`,
				{ token: user.accessToken, clientIp: user.clientIp },
			);
			expect(res.dataPoints.map((d) => d.value)).toContain('42.5');
			const reqs = await observabilityRequests(user.workspaceId);
			expect(reqs.find((r) => r.kind === 'mimir')?.headers['x-scope-orgid']).toBe(
				user.workspaceId,
			);
		}
	});

	test('an unknown metric name is rejected (4xx)', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await expect(
			api.request('GET', `${base(user.workspaceId)}/${runner.id}/metrics/not_a_metric`, {
				token: user.accessToken,
				clientIp: user.clientIp,
			}),
		).rejects.toThrow(/40[0-9]/);
	});

	test('a malformed Mimir response surfaces a 500', async ({ api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, { malformed: 'mimir' });
		await expect(
			api.request('GET', `${base(user.workspaceId)}/${runner.id}/metrics/system_cpu_usage`, {
				token: user.accessToken,
				clientIp: user.clientIp,
			}),
		).rejects.toThrow(/500/);
	});
});

// The detail Logs/Metrics tabs are the user-facing surface; the LogQL/PromQL/
// org-header precision above has no UI equivalent and stays API-only.
test.describe('runner > logs/metrics [UI]', () => {
	test('the Logs tab renders seeded log lines', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, {
			loki: { values: [['1700000000000000000', 'runner-log-ui-marker']] },
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerDetail(page, runner.id, 'logs');
			await expect(page.getByText('runner-log-ui-marker')).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('the Metrics tab renders its chart cards', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await resetObservability(user.workspaceId);
		await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '42']] } });
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerDetail(page, runner.id, 'metrics');
			await expect(metricsTab(page)).toBeVisible({ timeout: 15_000 });
			// The metrics view renders its chart cards (CPU is the first).
			await expect(page.getByText('CPU', { exact: true })).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});

test.describe('runner > logs/metrics RBAC [API]', () => {
	async function permId(
		api: ApiClient,
		owner: UserHandle & { workspaceId: string },
		name: string,
	) {
		return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
	}

	test('logs and metrics require runner::view', async ({ api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		// Member with create (not view) — should be denied logs/metrics.
		const createId = await permId(api, owner, 'runner::create');
		await using member = await createSecondMemberWithRole(api, owner, {
			[createId]: { permissionType: 'exclude', resources: [] },
		});
		await expect(
			api.request('GET', `${base(owner.workspaceId)}/${runner.id}/logs`, {
				token: member.accessToken,
				clientIp: member.clientIp,
			}),
		).rejects.toThrow(/401/);
		await expect(
			api.request('GET', `${base(owner.workspaceId)}/${runner.id}/metrics/system_cpu_usage`, {
				token: member.accessToken,
				clientIp: member.clientIp,
			}),
		).rejects.toThrow(/401/);
	});
});
