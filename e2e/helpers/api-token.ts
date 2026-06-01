import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

// Mirrors models/src/rbac/workspace_permission.rs — tagged enum, camelCase.
export type WorkspacePermissionInput =
  | { type: 'superAdmin' }
  | ({ type: 'member' } & Record<
      string,
      { permissionType: 'include' | 'exclude'; resources: string[] }
    >);

export type CreateApiTokenOpts = {
  name?: string;
  permissions: Record<string, WorkspacePermissionInput>;
  tokenNbf?: Date | null;
  tokenExp?: Date | null;
  allowedIps?: string[];
};

export type ApiTokenHandle = {
  id: string;
  token: string; // patrv1.<refresh>.<loginId>
  name: string;
};

export async function createApiTokenAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  opts: CreateApiTokenOpts,
): Promise<ApiTokenHandle> {
  const name =
    opts.name ?? `tkn-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
  // The `created` field has a serde default and is rejected if sent explicitly
  // (preprocess validates the body shape strictly). Omit it.
  const body: Record<string, unknown> = {
    name,
    permissions: opts.permissions,
  };
  if (opts.tokenNbf !== undefined && opts.tokenNbf !== null) body.tokenNbf = opts.tokenNbf;
  if (opts.tokenExp !== undefined && opts.tokenExp !== null) body.tokenExp = opts.tokenExp;
  if (opts.allowedIps && opts.allowedIps.length > 0) body.allowedIps = opts.allowedIps;

  const resp = await api.request<{ id: string; token: string }>('POST', '/user/api-token', {
    token: user.accessToken,
    clientIp: user.clientIp,
    body,
  });
  return { id: resp.id, token: resp.token, name };
}

export async function patchApiTokenAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  id: string,
  patch: Partial<{
    name: string;
    permissions: Record<string, WorkspacePermissionInput>;
    tokenNbf: Date | null;
    tokenExp: Date | null;
    allowedIps: string[];
  }>,
): Promise<void> {
  await api.request('PATCH', `/user/api-token/${id}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: patch,
  });
}

// Calls a known authed endpoint with the raw API token as Bearer. Returns
// status without throwing so tests can assert 200 vs 401 cleanly.
// Defaults to GET /user/workspaces which any token with at least one workspace
// permission should be able to call.
//
// API tokens use the api.patr.cloud entrypoint directly (see API_DIRECT_URL).
// The /api proxy on DASHBOARD_URL refuses Bearer tokens with 400 because it
// expects the authState cookie.
export async function callWithApiToken(
  _api: ApiClient,
  token: string,
  opts: {
    clientIp?: string;
    path?: string;
    method?: string;
    body?: unknown;
  } = {},
): Promise<{ status: number; ok: boolean; body: unknown }> {
  const path = opts.path ?? '/user/workspaces';
  const method = opts.method ?? 'GET';
  const headers: Record<string, string> = {
    Authorization: `Bearer ${token}`,
    'User-Agent': USER_AGENT,
    ...(opts.body !== undefined ? { 'Content-Type': 'application/json' } : {}),
  };
  if (opts.clientIp) headers['X-Real-IP'] = opts.clientIp;
  const res = await fetch(`${API_DIRECT_URL}${path}`, {
    method,
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
  });
  const text = await res.text();
  let body: unknown = text;
  try {
    body = text ? JSON.parse(text) : undefined;
  } catch {
    // leave as raw text
  }
  return { status: res.status, ok: res.ok, body };
}
