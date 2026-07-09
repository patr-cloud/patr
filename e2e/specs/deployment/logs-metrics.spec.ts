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
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, DEPLOYMENT_METRIC_NAMES } from '@/helpers/deployment-api';
import { openDeploymentDetail, metricsTab } from '@/helpers/ui/deployment';
import {
  configureObservability,
  observabilityRequests,
  resetObservability,
} from '@/helpers/observability';

// Deployment logs come from Loki (LogQL `{deployment_id="<id>"}`), metrics from
// Mimir. The e2e stack points both at the observability stub keyed by
// x-scope-orgid = workspace id. A deployment row (API-created) is enough to
// drive the read path — no runner needed.

test.beforeAll(async () => {
  await seedMachineType();
});

async function setup(api: ApiClient) {
  const user = await createUserWithWorkspace(api);
  const runner = await createRunnerAPI(api, user, user.workspaceId);
  const repo = await createContainerRepo(api, user, user.workspaceId);
  const dep = await createDeploymentAPI(api, user, user.workspaceId, {
    repositoryId: repo.id,
    runnerId: runner.id,
  });
  return { user, dep };
}

const base = (ws: string) => `/workspace/${ws}/deployment`;

test.describe('deployment > logs [API]', () => {
  test('serves logs and records the LogQL + org header', async ({ api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, {
      loki: { values: [['1700000000000000000', 'container started']] },
    });
    const res = await api.request<{ logs: Array<{ log: string }> }>(
      'GET',
      `${base(user.workspaceId)}/${dep.id}/logs`,
      { token: user.accessToken, clientIp: user.clientIp },
    );
    expect(res.logs.map((l) => l.log)).toContain('container started');

    const loki = (await observabilityRequests(user.workspaceId)).find((r) => r.kind === 'loki');
    expect(loki?.headers['x-scope-orgid']).toBe(user.workspaceId);
    expect(loki?.query.query).toContain(dep.id);
    // Default limit is 100.
    expect(loki?.query.limit).toBe('100');
  });

  test('passes a search filter through to the LogQL query', async ({ api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { loki: { values: [] } });
    await api.request('GET', `${base(user.workspaceId)}/${dep.id}/logs?search=panic`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    const loki = (await observabilityRequests(user.workspaceId)).find((r) => r.kind === 'loki');
    expect(loki?.query.query).toContain('panic');
  });

  test('a limit above 500 is rejected (400)', async ({ api }) => {
    const { user, dep } = await setup(api);
    await expect(
      api.request('GET', `${base(user.workspaceId)}/${dep.id}/logs?limit=501`, {
        token: user.accessToken,
        clientIp: user.clientIp,
      }),
    ).rejects.toThrow(/400/);
  });

  test('a malformed Loki response surfaces a 500', async ({ api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { malformed: 'loki' });
    await expect(
      api.request('GET', `${base(user.workspaceId)}/${dep.id}/logs`, {
        token: user.accessToken,
        clientIp: user.clientIp,
      }),
    ).rejects.toThrow(/500/);
  });
});

test.describe('deployment > metrics [API]', () => {
  test('serves each of the 26 metric names with the org header', async ({ api }) => {
    expect(DEPLOYMENT_METRIC_NAMES).toHaveLength(26);
    // The per-second rate limit is 50 req/s per login in debug builds; keep
    // the 26 names split across two fresh workspaces (separate rate buckets)
    // anyway for headroom.
    const half = Math.ceil(DEPLOYMENT_METRIC_NAMES.length / 2);
    const chunks = [DEPLOYMENT_METRIC_NAMES.slice(0, half), DEPLOYMENT_METRIC_NAMES.slice(half)];
    for (const names of chunks) {
      const { user, dep } = await setup(api);
      await resetObservability(user.workspaceId);
      await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '7']] } });
      for (const metric of names) {
        const res = await api.request<{ dataPoints: Array<{ value: string }> }>(
          'GET',
          `${base(user.workspaceId)}/${dep.id}/metrics/${metric}`,
          { token: user.accessToken, clientIp: user.clientIp },
        );
        expect(res.dataPoints.map((d) => d.value)).toContain('7');
      }
      const mimirReqs = (await observabilityRequests(user.workspaceId)).filter(
        (r) => r.kind === 'mimir',
      );
      expect(mimirReqs.length).toBe(names.length);
      expect(mimirReqs.every((r) => r.headers['x-scope-orgid'] === user.workspaceId)).toBe(true);
      expect(mimirReqs.every((r) => r.query.query.includes(dep.id))).toBe(true);
    }
  });

  test('the default interval maps to a 2m step', async ({ api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '1']] } });
    await api.request('GET', `${base(user.workspaceId)}/${dep.id}/metrics/ingress_rps`, {
      token: user.accessToken,
      clientIp: user.clientIp,
    });
    const mimir = (await observabilityRequests(user.workspaceId)).find((r) => r.kind === 'mimir');
    expect(mimir?.query.step).toBe('2m');
  });

  test('larger intervals widen the step (6h→5m, 7d→1h)', async ({ api }) => {
    const { user, dep } = await setup(api);
    // interval is a float number of seconds (the metrics UI sends `<secs>.0`).
    for (const [seconds, step] of [
      [21_600, '5m'], // 6h
      [604_800, '1h'], // 7d
    ] as const) {
      await resetObservability(user.workspaceId);
      await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '1']] } });
      await api.request(
        'GET',
        `${base(user.workspaceId)}/${dep.id}/metrics/ingress_rps?interval=${seconds}.0`,
        { token: user.accessToken, clientIp: user.clientIp },
      );
      const mimir = (await observabilityRequests(user.workspaceId)).find((r) => r.kind === 'mimir');
      expect(mimir?.query.step).toBe(step);
    }
  });

  test('an interval beyond 14 days is rejected (400)', async ({ api }) => {
    const { user, dep } = await setup(api);
    await expect(
      api.request(
        'GET',
        `${base(user.workspaceId)}/${dep.id}/metrics/ingress_rps?interval=${15 * 24 * 3600}.0`,
        { token: user.accessToken, clientIp: user.clientIp },
      ),
    ).rejects.toThrow(/400/);
  });

  test('an unknown metric name is rejected (4xx)', async ({ api }) => {
    const { user, dep } = await setup(api);
    await expect(
      api.request('GET', `${base(user.workspaceId)}/${dep.id}/metrics/not_a_metric`, {
        token: user.accessToken,
        clientIp: user.clientIp,
      }),
    ).rejects.toThrow(/40[0-9]/);
  });

  test('a malformed Mimir response surfaces a 500', async ({ api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { malformed: 'mimir' });
    await expect(
      api.request('GET', `${base(user.workspaceId)}/${dep.id}/metrics/container_cpu_usage`, {
        token: user.accessToken,
        clientIp: user.clientIp,
      }),
    ).rejects.toThrow(/500/);
  });
});

// The detail Logs/Metrics tabs are the user-facing surface; the LogQL/PromQL/
// step/org-header precision above has no UI equivalent and stays API-only.
test.describe('deployment > logs/metrics [UI]', () => {
  test('the Logs tab renders seeded log lines', async ({ browser, api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, {
      loki: { values: [['1700000000000000000', 'deployment-log-ui-marker']] },
    });
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentDetail(page, dep.id, 'logs');
      await expect(page.getByText('deployment-log-ui-marker')).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });

  test('the Metrics tab renders its chart cards', async ({ browser, api }) => {
    const { user, dep } = await setup(api);
    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '42']] } });
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openDeploymentDetail(page, dep.id, 'metrics');
      await expect(metricsTab(page)).toBeVisible({ timeout: 15_000 });
      await expect(page.getByText('CPU', { exact: true })).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });
});

test.describe('deployment > logs/metrics RBAC [API]', () => {
  async function permId(api: ApiClient, owner: UserHandle & { workspaceId: string }, name: string) {
    return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
  }

  test('logs and metrics require deployment::view', async ({ api }) => {
    const { user: owner, dep } = await setup(api);
    const createId = await permId(api, owner, 'deployment::create');
    await using member = await createSecondMemberWithRole(api, owner, {
      [createId]: { permissionType: 'exclude', resources: [] },
    });
    await expect(
      api.request('GET', `${base(owner.workspaceId)}/${dep.id}/logs`, {
        token: member.accessToken,
        clientIp: member.clientIp,
      }),
    ).rejects.toThrow(/401/);
    await expect(
      api.request('GET', `${base(owner.workspaceId)}/${dep.id}/metrics/ingress_rps`, {
        token: member.accessToken,
        clientIp: member.clientIp,
      }),
    ).rejects.toThrow(/401/);
  });
});
