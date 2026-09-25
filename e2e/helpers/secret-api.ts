import type { ApiClient } from '@/helpers/api';

// REST helpers for workspace secrets. Only metadata comes back from the API —
// values live in OpenBao and are never returned by these routes, so tests can
// assert a secret exists and what it is called, but never what it holds.

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/secret`;

export type Secret = {
	id: string;
	name: string;
	created: string;
	lastUpdated: string;
};

export function randomSecretName(prefix = 'E2ESECRET'): string {
	return `${prefix}_${crypto.randomUUID().replace(/-/g, '').slice(0, 8).toUpperCase()}`;
}

export async function createSecretAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	name: string,
	value: string,
): Promise<{ id: string }> {
	return api.request<{ id: string }>('POST', base(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: { name, value },
	});
}

export async function listSecretsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
): Promise<Secret[]> {
	const response = await api.request<{ secrets: Secret[] }>('GET', base(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
	return response.secrets ?? [];
}

/** The secret with this name, or undefined when the workspace has no such secret. */
export async function findSecretByName(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	name: string,
): Promise<Secret | undefined> {
	const secrets = await listSecretsAPI(api, user, workspaceId);
	return secrets.find((secret) => secret.name === name);
}

export async function getSecretAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	secretId: string,
): Promise<Secret> {
	const response = await api.request<{ secret: Secret }>(
		'GET',
		`${base(workspaceId)}/${secretId}`,
		{
			token: user.accessToken,
			clientIp: user.clientIp,
		},
	);
	return response.secret;
}

export async function deleteSecretAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	secretId: string,
): Promise<void> {
	await api.request('DELETE', `${base(workspaceId)}/${secretId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

/** Rename a secret, and rotate its value when one is given. */
export async function updateSecretAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	secretId: string,
	body: { name: string; value?: string },
): Promise<void> {
	await api.request('PATCH', `${base(workspaceId)}/${secretId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
		body,
	});
}
