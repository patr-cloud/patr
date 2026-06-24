import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL, DASHBOARD_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

// REST helpers for the runner feature, kept separate from the heavy DinD
// `RunnerHandle` (helpers/runner.ts) so non-@docker specs don't pull in the
// child-process / docker machinery.

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/runner`;

// Runner names go through RESOURCE_NAME_REGEX (4-255, allows upper/space/dot);
// unlike container repos there is NO stricter DB CHECK. Default fixture names
// are simple and unique.
export function randomRunnerName(prefix = 'e2e-runner'): string {
  return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

export type RunnerInfo = {
  id: string;
  name: string;
  connected: boolean;
  lastSeen: string | null;
};

export async function createRunnerAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  name?: string,
): Promise<{ id: string; name: string }> {
  const runnerName = name ?? randomRunnerName();
  const resp = await api.request<{ id: string }>('POST', base(workspaceId), {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: { name: runnerName },
  });
  return { id: resp.id, name: runnerName };
}

export async function getRunnerInfoAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  runnerId: string,
): Promise<RunnerInfo> {
  const resp = await api.request<{ runner: RunnerInfo }>(
    'GET',
    `${base(workspaceId)}/${runnerId}`,
    { token: user.accessToken, clientIp: user.clientIp },
  );
  return resp.runner;
}

export async function deleteRunnerAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  runnerId: string,
): Promise<void> {
  await api.request('DELETE', `${base(workspaceId)}/${runnerId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}

// Lists runners, returning rows + the x-total-count header. User JWTs go
// through the dashboard proxy; API tokens must use the direct entrypoint
// (`direct: true`). Mirrors listReposAPI.
export async function listRunnersAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  query = '',
  opts: { direct?: boolean } = {},
): Promise<{ runners: RunnerInfo[]; totalCount: number | null }> {
  const baseUrl = opts.direct ? API_DIRECT_URL : `${DASHBOARD_URL}/api`;
  const res = await fetch(`${baseUrl}${base(workspaceId)}${query}`, {
    headers: {
      'X-Real-IP': user.clientIp,
      'User-Agent': USER_AGENT,
      Authorization: `Bearer ${user.accessToken}`,
    },
  });
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`listRunnersAPI → ${res.status}: ${text.slice(0, 300)}`);
  }
  const header = res.headers.get('x-total-count');
  const body = JSON.parse(text) as { runners: RunnerInfo[] };
  return { runners: body.runners, totalCount: header === null ? null : Number(header) };
}

export async function getIngressTokenAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  runnerId: string,
): Promise<string> {
  const resp = await api.request<{ token: string }>(
    'GET',
    `${base(workspaceId)}/${runnerId}/ingress-token`,
    { token: user.accessToken, clientIp: user.clientIp },
  );
  return resp.token;
}
