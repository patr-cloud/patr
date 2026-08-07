import type { ApiClient } from '@/helpers/api';
import { createApiTokenAPI } from '@/helpers/api-token';
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

type RunnerLink = {
	userCode: string;
	deviceCode: string;
};

// Open a consent link the way the CLI does. `create-link` is `[ApiToken]`-only,
// so it must go to the direct entrypoint with a Bearer API token — the
// dashboard proxy won't serve it.
export async function openRunnerLinkAPI(
	user: Creds,
	workspaceId: string,
	apiToken: string,
	hostname?: string,
): Promise<RunnerLink> {
	const res = await fetch(`${API_DIRECT_URL}${base(workspaceId)}/link`, {
		method: 'POST',
		headers: {
			'X-Real-IP': user.clientIp,
			'User-Agent': USER_AGENT,
			'Content-Type': 'application/json',
			Authorization: `Bearer ${apiToken}`,
		},
		body: JSON.stringify({
			version: '0.1.0',
			os: 'linux',
			arch: 'x86_64',
			hostname: hostname ?? randomRunnerName('e2e-host'),
			privateIp: '127.0.0.1',
		}),
	});
	const text = await res.text();
	if (!res.ok) {
		throw new Error(`openRunnerLinkAPI → ${res.status}: ${text.slice(0, 300)}`);
	}
	return JSON.parse(text) as RunnerLink;
}

// Claim the credentials the way the CLI's verify poll does. Returns the
// runner's service account token once the link has been approved.
export async function verifyRunnerLinkAPI(
	user: Creds,
	workspaceId: string,
	apiToken: string,
	link: RunnerLink,
): Promise<{ runnerId: string; token: string }> {
	const res = await fetch(`${API_DIRECT_URL}${base(workspaceId)}/link/verify`, {
		method: 'POST',
		headers: {
			'X-Real-IP': user.clientIp,
			'User-Agent': USER_AGENT,
			'Content-Type': 'application/json',
			Authorization: `Bearer ${apiToken}`,
		},
		body: JSON.stringify({ userCode: link.userCode, deviceCode: link.deviceCode }),
	});
	const text = await res.text();
	if (!res.ok) {
		throw new Error(`verifyRunnerLinkAPI → ${res.status}: ${text.slice(0, 300)}`);
	}
	// The result enum is `#[serde(flatten)]`ed into the response, so the wire
	// shape is `{status, runnerId, workspaceId, token}` — not nested under a
	// `result` key.
	const body = JSON.parse(text) as
		{ status: 'approved'; runnerId: string; token: string } | { status: 'pending' };
	if (body.status !== 'approved') {
		throw new Error('verifyRunnerLinkAPI: link is still pending');
	}
	return { runnerId: body.runnerId, token: body.token };
}

// Create a runner end-to-end through the consent-link flow, the way the CLI +
// browser do it. There is no longer a direct "create runner" endpoint: the
// runner, its role and its service account are all minted by `approve`.
//
// `approve` is `[WebDashboard]`-only so it goes through the dashboard client on
// the user's session, while `create-link` and `verify` are `[ApiToken]`-only
// and need the API token minted here.
export async function createRunnerAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	name?: string,
): Promise<{ id: string; name: string; token: string }> {
	const runnerName = name ?? randomRunnerName();

	const apiToken = await createApiTokenAPI(api, user, {
		permissions: { [workspaceId]: { type: 'superAdmin' } },
	});

	const link = await openRunnerLinkAPI(user, workspaceId, apiToken.token, runnerName);

	await api.request('POST', `${base(workspaceId)}/link/${link.userCode}/approve`, {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: { runnerName },
	});

	const { runnerId, token } = await verifyRunnerLinkAPI(user, workspaceId, apiToken.token, link);

	return { id: runnerId, name: runnerName, token };
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
