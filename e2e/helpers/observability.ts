// Test-side control plane for the Loki/Mimir stub (mocks/observability.mjs).
// The e2e stack points the API's logs/metrics endpoints at the stub; these
// helpers configure the responses it returns for a given workspace (org) and
// read back the requests the API made (to assert the LogQL/PromQL, step/limit,
// and the x-scope-orgid header). Unconfigured workspaces are proxied to the
// real Loki/Mimir, so leaving a workspace unconfigured exercises the real path.

const OBS_MOCK_PORT = Number(process.env.OBS_MOCK_PORT ?? 13900);
const OBS_MOCK_URL = `http://127.0.0.1:${OBS_MOCK_PORT}`;

export type ObservabilityConfig = {
  // Loki query_range result values: [unixNanos, logLine].
  loki?: { values: Array<[string, string]> };
  // Mimir query_range result values: [unixSeconds, value].
  mimir?: { values: Array<[number, string]> };
  // Loki tail frames pushed over the websocket; each frame is a values array.
  tail?: Array<Array<[string, string]>>;
  // Return un-parseable body for this backend so the API's parse-error (500)
  // path can be exercised.
  malformed?: 'loki' | 'mimir';
};

export type RecordedRequest = {
  kind: 'loki' | 'mimir' | 'loki-tail';
  path: string;
  query: Record<string, string>;
  headers: Record<string, string>;
};

export async function configureObservability(org: string, cfg: ObservabilityConfig): Promise<void> {
  await fetch(`${OBS_MOCK_URL}/__configure`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ org, ...cfg }),
  });
}

export async function observabilityRequests(org: string): Promise<RecordedRequest[]> {
  const res = await fetch(`${OBS_MOCK_URL}/__requests?org=${encodeURIComponent(org)}`);
  return (await res.json()) as RecordedRequest[];
}

export async function resetObservability(org: string): Promise<void> {
  await fetch(`${OBS_MOCK_URL}/__reset?org=${encodeURIComponent(org)}`, { method: 'POST' });
}
