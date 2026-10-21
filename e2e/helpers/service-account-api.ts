import type { ApiClient } from '@/helpers/api';
import { listRolesAPI, toGrants, type RoleGrant } from '@/helpers/api/rbac';
import { USER_AGENT } from '@/helpers/config';
import { DASHBOARD_URL } from '@/helpers/urls';

// REST helpers for workspace service accounts. A service account's token comes
// back only from create and regenerate; list and get never return it.

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/service-account`;

// Names go through RESOURCE_NAME_REGEX (2-255, letters, digits, space, dot,
// hyphen, underscore) and are unique per workspace.
export function randomServiceAccountName(prefix = 'e2e-sa'): string {
	return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

export type ServiceAccount = { id: string; name: string; description: string | null };

export type ServiceAccountInfo = ServiceAccount & { roleBindings: RoleGrant[] };

// Bare role ids are granted at the workspace root; pass full RoleGrants to
// scope a role to specific resources.
export async function createServiceAccountAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	opts: { roles: (string | RoleGrant)[]; name?: string; description?: string | null },
): Promise<{ id: string; name: string; token: string }> {
	const name = opts.name ?? randomServiceAccountName();
	const resp = await api.request<{ id: string; token: string }>('POST', base(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: {
			name,
			description: opts.description ?? null,
			roleBindings: toGrants(workspaceId, opts.roles),
		},
	});
	return { id: resp.id, name, token: resp.token };
}

export async function getServiceAccountAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	serviceAccountId: string,
): Promise<ServiceAccountInfo> {
	return api.request<ServiceAccountInfo>('GET', `${base(workspaceId)}/${serviceAccountId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

// Lists service accounts, returning the raw body alongside the rows so specs
// can check what the response carries, plus the x-total-count header.
export async function listServiceAccountsAPI(
	user: Creds,
	workspaceId: string,
	query = '',
): Promise<{ serviceAccounts: ServiceAccount[]; totalCount: number | null; raw: string }> {
	const res = await fetch(`${DASHBOARD_URL}/api${base(workspaceId)}${query}`, {
		headers: {
			'X-Real-IP': user.clientIp,
			'User-Agent': USER_AGENT,
			Authorization: `Bearer ${user.accessToken}`,
		},
	});
	const raw = await res.text();
	if (!res.ok) {
		throw new Error(`listServiceAccountsAPI → ${res.status}: ${raw.slice(0, 300)}`);
	}
	const header = res.headers.get('x-total-count');
	const body = JSON.parse(raw) as { serviceAccounts: ServiceAccount[] };
	return {
		serviceAccounts: body.serviceAccounts,
		totalCount: header === null ? null : Number(header),
		raw,
	};
}

// The endpoint is a full replace: the name, the description and every grant.
export async function updateServiceAccountAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	serviceAccountId: string,
	body: { name: string; description?: string | null; roles: (string | RoleGrant)[] },
): Promise<void> {
	await api.request('PATCH', `${base(workspaceId)}/${serviceAccountId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: {
			name: body.name,
			description: body.description ?? null,
			roleBindings: toGrants(workspaceId, body.roles),
		},
	});
}

// Rotate a service account's token. The old token stops working at once.
export async function regenerateServiceAccountTokenAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	serviceAccountId: string,
): Promise<string> {
	const resp = await api.request<{ token: string }>(
		'POST',
		`${base(workspaceId)}/${serviceAccountId}/token`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.token;
}

export async function deleteServiceAccountAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	serviceAccountId: string,
): Promise<void> {
	await api.request('DELETE', `${base(workspaceId)}/${serviceAccountId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

// Every workspace is seeded with built-in roles such as "Runner: Viewer" and
// "Service Account: Editor". Resolves one by name.
export async function builtInRoleId(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	name: string,
): Promise<string> {
	const roles = await listRolesAPI(api, user, workspaceId);
	const role = roles.find((r) => r.name === name);
	if (!role) throw new Error(`No built-in role named "${name}"`);
	return role.id;
}
