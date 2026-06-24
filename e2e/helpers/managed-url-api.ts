import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL, DASHBOARD_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';
import { addDomainAPI, randomDomain } from '@/helpers/domain-api';
import { markDomainVerified } from '@/helpers/db';

// REST helpers for managed URLs. Managed URLs hang off a (verified) domain and
// every create/update/delete/verify call hits Cloudflare (the e2e CF mock).
// The route base is /infrastructure/managed-url (workspace-wide, not under the
// domain). Update is POST (the frontend's PATCH is a 405 bug — pinned).

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/infrastructure/managed-url`;

export function randomSubdomain(prefix = 'app'): string {
  return `${prefix}${crypto.randomUUID().replace(/-/g, '').slice(0, 8)}`;
}

// Add an external domain and flip it verified via the DB backdoor (real DNS
// verification can't succeed in e2e). Returns the domain id + full name.
export async function createVerifiedDomain(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  domain?: string,
): Promise<{ id: string; domain: string }> {
  const added = await addDomainAPI(api, user, workspaceId, domain ?? randomDomain());
  await markDomainVerified(added.id);
  return added;
}

export type ManagedUrl = {
  id: string;
  subDomain: string;
  domainId: string;
  path: string;
  type: string;
  isActive: boolean;
  deploymentId?: string;
  port?: number;
  url?: string;
};

export type ProxyDeploymentOpts = {
  domainId: string;
  deploymentId: string;
  port: number;
  subDomain?: string;
  path?: string;
};

export function proxyDeploymentBody(opts: ProxyDeploymentOpts): Record<string, unknown> {
  return {
    subDomain: opts.subDomain ?? randomSubdomain(),
    domainId: opts.domainId,
    path: opts.path ?? '/',
    type: 'proxyDeployment',
    deploymentId: opts.deploymentId,
    port: opts.port,
  };
}

export async function createManagedUrlAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  body: Record<string, unknown>,
): Promise<{ id: string }> {
  return api.request<{ id: string }>('POST', base(workspaceId), {
    token: user.accessToken,
    clientIp: user.clientIp,
    body,
  });
}

// POST a (possibly invalid) body and return the numeric HTTP status (201 on
// success). Mirrors the deployment validation helper.
export async function createManagedUrlStatus(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  body: Record<string, unknown>,
): Promise<number> {
  try {
    await api.request('POST', base(workspaceId), {
      token: user.accessToken,
      clientIp: user.clientIp,
      body,
    });
    return 201;
  } catch (err) {
    const m = String(err).match(/→ (\d+)/);
    if (!m) throw err;
    return Number(m[1]);
  }
}

export async function listManagedUrlsAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  query = '',
  opts: { direct?: boolean } = {},
): Promise<{ urls: ManagedUrl[]; totalCount: number | null }> {
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
    throw new Error(`listManagedUrlsAPI → ${res.status}: ${text.slice(0, 300)}`);
  }
  const header = res.headers.get('x-total-count');
  const body = JSON.parse(text) as { urls: ManagedUrl[] };
  return { urls: body.urls, totalCount: header === null ? null : Number(header) };
}

// Update is POST (the frontend uses PATCH → 405).
export async function updateManagedUrlAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  managedUrlId: string,
  body: Record<string, unknown>,
): Promise<void> {
  await api.request('POST', `${base(workspaceId)}/${managedUrlId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body,
  });
}

export async function deleteManagedUrlAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  managedUrlId: string,
): Promise<void> {
  await api.request('DELETE', `${base(workspaceId)}/${managedUrlId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}

export async function verifyConfigurationAPI(
  api: ApiClient,
  user: Creds,
  workspaceId: string,
  managedUrlId: string,
): Promise<boolean> {
  const resp = await api.request<{ configured: boolean }>(
    'POST',
    `${base(workspaceId)}/${managedUrlId}/verify-configuration`,
    { token: user.accessToken, clientIp: user.clientIp },
  );
  return resp.configured;
}
