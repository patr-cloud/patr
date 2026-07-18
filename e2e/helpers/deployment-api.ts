import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL, DASHBOARD_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';
import { DEFAULT_MACHINE_TYPE_ID } from '@/helpers/db';

// REST helpers for the deployment feature, kept separate from the heavy DinD
// execution helpers (helpers/deployment.ts) so the non-@docker breadth specs
// don't pull in the docker machinery. Mirrors runner-api.ts.

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/deployment`;

export const PATR_REGISTRY = 'registry.patr.cloud';

// Deployment names go through RESOURCE_NAME_REGEX (4-255, allows upper/space/
// dot); there is NO uniqueness constraint, so duplicates are allowed. The
// create form has no client-side name validation — every name is POSTed.
export function randomDeploymentName(prefix = 'e2e-dep'): string {
	return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

export type ExposedPortType = 'http' | 'tcp' | 'udp';

export type DeploymentInfo = {
	id: string;
	name: string;
	registry: string;
	repositoryId?: string;
	imageName?: string;
	imageTag: string;
	status: string;
	runner: string;
	machineType: string;
	currentLiveDigest: string | null;
	deployOnPush: boolean;
	minHorizontalScale: number;
	maxHorizontalScale: number;
	ports: Record<string, ExposedPortType>;
	environmentVariables: Record<string, unknown>;
	startupProbe?: { port: number; path: string };
	livenessProbe?: { port: number; path: string };
	configMounts: Record<string, string>;
	volumes: Record<string, string>;
};

// The body the create endpoint expects. registry + running_details are
// `#[serde(flatten)]`-ed onto the top level, so this is one flat object.
export type CreateDeploymentBody = {
	name: string;
	registry: string;
	repositoryId?: string;
	imageName?: string;
	imageTag: string;
	runner: string;
	machineType: string;
	deployOnPush: boolean;
	minHorizontalScale: number;
	maxHorizontalScale: number;
	ports: Record<string, ExposedPortType>;
	environmentVariables?: Record<string, unknown>;
	startupProbe?: { port: number; path: string };
	livenessProbe?: { port: number; path: string };
	configMounts?: Record<string, string>;
	volumes?: Record<string, string>;
	deployOnCreate: boolean;
};

export type CreateDeploymentOpts = {
	repositoryId: string;
	runnerId: string;
	imageTag?: string;
	name?: string;
	port?: number;
	ports?: Record<string, ExposedPortType>;
	deployOnCreate?: boolean;
	deployOnPush?: boolean;
	minHorizontalScale?: number;
	maxHorizontalScale?: number;
	environmentVariables?: Record<string, unknown>;
	startupProbe?: { port: number; path: string };
	livenessProbe?: { port: number; path: string };
	configMounts?: Record<string, string>;
	volumes?: Record<string, string>;
};

// Build a default Patr-registry create body from opts. deployOnCreate defaults
// to false (matches the create form) so the runner doesn't try to pull unless
// asked.
export function patrDeploymentBody(opts: CreateDeploymentOpts): CreateDeploymentBody {
	const ports = opts.ports ?? { [String(opts.port ?? 80)]: 'http' as const };
	return {
		name: opts.name ?? randomDeploymentName(),
		registry: PATR_REGISTRY,
		repositoryId: opts.repositoryId,
		imageTag: opts.imageTag ?? 'latest',
		runner: opts.runnerId,
		machineType: DEFAULT_MACHINE_TYPE_ID,
		deployOnPush: opts.deployOnPush ?? false,
		minHorizontalScale: opts.minHorizontalScale ?? 1,
		maxHorizontalScale: opts.maxHorizontalScale ?? 1,
		ports,
		environmentVariables: opts.environmentVariables,
		startupProbe: opts.startupProbe,
		livenessProbe: opts.livenessProbe,
		configMounts: opts.configMounts,
		volumes: opts.volumes,
		deployOnCreate: opts.deployOnCreate ?? false,
	};
}

// External (docker.io) variant — deployOnPush is forced false (the runner only
// honors deploy-on-push for the Patr registry).
export function externalDeploymentBody(opts: {
	runnerId: string;
	imageName?: string;
	imageTag?: string;
	name?: string;
	port?: number;
	deployOnCreate?: boolean;
}): CreateDeploymentBody {
	return {
		name: opts.name ?? randomDeploymentName(),
		registry: 'docker.io',
		imageName: opts.imageName ?? 'traefik/whoami',
		imageTag: opts.imageTag ?? 'latest',
		runner: opts.runnerId,
		machineType: DEFAULT_MACHINE_TYPE_ID,
		deployOnPush: false,
		minHorizontalScale: 1,
		maxHorizontalScale: 1,
		ports: { [String(opts.port ?? 80)]: 'http' },
		deployOnCreate: opts.deployOnCreate ?? false,
	};
}

export async function createDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	opts: CreateDeploymentOpts,
): Promise<{ id: string; name: string }> {
	const body = patrDeploymentBody(opts);
	const resp = await api.request<{ id: string }>('POST', base(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
		body,
	});
	return { id: resp.id, name: body.name };
}

// POST a (possibly invalid) body and return the numeric HTTP status. Mirrors
// the domain spec's `addStatus` — for validation tests that assert a status
// code rather than a thrown error. 201 on success.
export async function createDeploymentStatus(
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

export async function getDeploymentInfoAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
): Promise<DeploymentInfo> {
	return api.request<DeploymentInfo>('GET', `${base(workspaceId)}/${deploymentId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export async function updateDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	body: Record<string, unknown>,
): Promise<void> {
	await api.request('PATCH', `${base(workspaceId)}/${deploymentId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
		body,
	});
}

export async function startDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	forceRestart = false,
): Promise<void> {
	const q = forceRestart ? '?forceRestart=true' : '';
	await api.request('POST', `${base(workspaceId)}/${deploymentId}/start${q}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export async function stopDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
): Promise<void> {
	await api.request('POST', `${base(workspaceId)}/${deploymentId}/stop`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export async function deleteDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
): Promise<void> {
	await api.request('DELETE', `${base(workspaceId)}/${deploymentId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

// Lists deployments, returning rows + the x-total-count header. User JWTs go
// through the dashboard proxy; API tokens must use the direct entrypoint
// (`direct: true`). Mirrors listRunnersAPI.
export async function listDeploymentsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	query = '',
	opts: { direct?: boolean } = {},
): Promise<{ deployments: Array<DeploymentInfo>; totalCount: number | null }> {
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
		throw new Error(`listDeploymentsAPI → ${res.status}: ${text.slice(0, 300)}`);
	}
	const header = res.headers.get('x-total-count');
	const body = JSON.parse(text) as { deployments: DeploymentInfo[] };
	return { deployments: body.deployments, totalCount: header === null ? null : Number(header) };
}

// ---------- deploy history ----------

export type DeployHistoryEntry = { imageDigest: string; created: string };

export async function listDeployHistoryAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	query = '',
	opts: { direct?: boolean } = {},
): Promise<{ deploys: DeployHistoryEntry[]; totalCount: number | null }> {
	const baseUrl = opts.direct ? API_DIRECT_URL : `${DASHBOARD_URL}/api`;
	const res = await fetch(
		`${baseUrl}${base(workspaceId)}/${deploymentId}/deploy-history${query}`,
		{
			headers: {
				'X-Real-IP': user.clientIp,
				'User-Agent': USER_AGENT,
				Authorization: `Bearer ${user.accessToken}`,
			},
		},
	);
	const text = await res.text();
	if (!res.ok) {
		throw new Error(`listDeployHistoryAPI → ${res.status}: ${text.slice(0, 300)}`);
	}
	const header = res.headers.get('x-total-count');
	const body = JSON.parse(text) as { deploys: DeployHistoryEntry[] };
	return { deploys: body.deploys, totalCount: header === null ? null : Number(header) };
}

export async function revertDeploymentAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	imageDigest: string,
): Promise<void> {
	await api.request(
		'POST',
		`${base(workspaceId)}/${deploymentId}/deploy-history/${encodeURIComponent(imageDigest)}/revert`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
}

export async function deleteDeployHistoryAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	imageDigest: string,
): Promise<void> {
	await api.request(
		'DELETE',
		`${base(workspaceId)}/${deploymentId}/deploy-history/${encodeURIComponent(imageDigest)}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
}

// ---------- logs & metrics ----------

export async function getDeploymentLogsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	query = '',
): Promise<Array<{ timestamp: string; log: string }>> {
	const resp = await api.request<{ logs: Array<{ timestamp: string; log: string }> }>(
		'GET',
		`${base(workspaceId)}/${deploymentId}/logs${query}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.logs;
}

export async function getDeploymentMetricAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	deploymentId: string,
	metric: string,
	query = '',
): Promise<Array<{ timestamp: string; value: string }>> {
	const resp = await api.request<{ dataPoints: Array<{ timestamp: string; value: string }> }>(
		'GET',
		`${base(workspaceId)}/${deploymentId}/metrics/${metric}${query}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.dataPoints;
}

// All 26 deployment metric names (DeploymentMetricName, snake_case).
export const DEPLOYMENT_METRIC_NAMES = [
	'ingress_rps',
	'ingress_latency_p50',
	'ingress_latency_p95',
	'ingress_latency_p99',
	'ingress_ttfb_p50',
	'ingress_ttfb_p95',
	'ingress_ttfb_p99',
	'ingress_error_rate',
	'ingress_status2xx',
	'ingress_status3xx',
	'ingress_status4xx',
	'ingress_status5xx',
	'ingress_bandwidth_in',
	'ingress_bandwidth_out',
	'ingress_active_connections',
	'ingress_request_body_size',
	'ingress_response_body_size',
	'container_cpu_usage',
	'container_cpu_throttling',
	'container_memory_used',
	'container_memory_limit',
	'container_network_rx',
	'container_network_tx',
	'container_disk_read',
	'container_disk_write',
	'container_oom_kills',
] as const;
