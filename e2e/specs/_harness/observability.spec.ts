import { test, expect, createUserWithWorkspace } from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import {
  configureObservability,
  observabilityRequests,
  resetObservability,
} from '@/helpers/observability';

// Harness check for the Loki/Mimir stub. A runner (created via the API, no DinD
// needed) is enough to exercise the logs/metrics read path: configure canned
// data for the workspace, call the endpoint, and assert the data plus the
// recorded LogQL/PromQL + x-scope-orgid. Also covers the parse-error (500) path.
async function createRunner(
  api: ApiClient,
  user: UserHandle & { workspaceId: string },
): Promise<string> {
  const r = await api.request<{ id: string }>('POST', `/workspace/${user.workspaceId}/runner`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: { name: `obs-${crypto.randomUUID().slice(0, 8)}` },
  });
  return r.id;
}

test.describe('observability stub (loki/mimir)', () => {
  test('serves deterministic runner logs and records the query + org', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const runnerId = await createRunner(api, user);

    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, {
      loki: { values: [['1700000000000000000', 'hello from stub']] },
    });

    const res = await api.request<{ logs: Array<{ log: string }> }>(
      'GET',
      `/workspace/${user.workspaceId}/runner/${runnerId}/logs`,
      { token: user.accessToken, clientIp: user.clientIp },
    );
    expect(res.logs.map((l) => l.log)).toContain('hello from stub');

    const reqs = await observabilityRequests(user.workspaceId);
    const loki = reqs.find((r) => r.kind === 'loki');
    expect(loki?.headers['x-scope-orgid']).toBe(user.workspaceId);
    expect(loki?.query.query).toContain(runnerId);
  });

  test('serves deterministic runner metrics', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const runnerId = await createRunner(api, user);

    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { mimir: { values: [[1700000000, '42.5']] } });

    const res = await api.request<{ dataPoints: Array<{ value: string }> }>(
      'GET',
      `/workspace/${user.workspaceId}/runner/${runnerId}/metrics/system_cpu_usage`,
      { token: user.accessToken, clientIp: user.clientIp },
    );
    expect(res.dataPoints.map((d) => d.value)).toContain('42.5');
  });

  test('malformed Loki response surfaces a 500', async ({ api }) => {
    await using user = await createUserWithWorkspace(api);
    const runnerId = await createRunner(api, user);

    await resetObservability(user.workspaceId);
    await configureObservability(user.workspaceId, { malformed: 'loki' });

    await expect(
      api.request('GET', `/workspace/${user.workspaceId}/runner/${runnerId}/logs`, {
        token: user.accessToken,
        clientIp: user.clientIp,
      }),
    ).rejects.toThrow(/500/);
  });
});
