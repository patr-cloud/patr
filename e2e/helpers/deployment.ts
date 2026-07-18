import type { ApiClient } from '@/helpers/api';
import { waitFor } from '@/helpers/process';
import { DEFAULT_MACHINE_TYPE_ID } from '@/helpers/db';

// The Patr registry sentinel the API serializes for PatrRegistry deployments.
const PATR_REGISTRY = 'registry.patr.cloud';

export type CreateDeploymentOpts = {
	workspaceId: string;
	repositoryId: string;
	imageTag: string;
	runnerId: string;
	port?: number;
	deployOnCreate?: boolean;
	deployOnPush?: boolean;
	name?: string;
};

// Create a Patr-registry deployment via the API. Seeds a single HTTP port and
// the default (seeded) machine type. deployOnCreate defaults to true so the
// runner starts it immediately.
export async function createPatrDeployment(
	api: ApiClient,
	user: { accessToken: string; clientIp: string },
	opts: CreateDeploymentOpts,
): Promise<{ id: string; port: number; name: string }> {
	const port = opts.port ?? 80;
	const name = opts.name ?? `e2e-dep-${crypto.randomUUID().slice(0, 8)}`;
	const resp = await api.request<{ id: string }>(
		'POST',
		`/workspace/${opts.workspaceId}/deployment`,
		{
			token: user.accessToken,
			clientIp: user.clientIp,
			body: {
				name,
				registry: PATR_REGISTRY,
				repositoryId: opts.repositoryId,
				imageTag: opts.imageTag,
				runner: opts.runnerId,
				machineType: DEFAULT_MACHINE_TYPE_ID,
				deployOnPush: opts.deployOnPush ?? false,
				minHorizontalScale: 1,
				maxHorizontalScale: 1,
				ports: { [String(port)]: 'http' },
				deployOnCreate: opts.deployOnCreate ?? true,
			},
		},
	);
	return { id: resp.id, port, name };
}

// Poll the deployment until its status matches one of the targets. Status is
// flattened to the top level of the get-info response.
export async function waitForDeploymentStatus(
	api: ApiClient,
	user: { accessToken: string; clientIp: string },
	workspaceId: string,
	deploymentId: string,
	target: string | string[],
	opts: { timeoutMs?: number } = {},
): Promise<string> {
	const targets = Array.isArray(target) ? target : [target];
	let last = '';
	await waitFor(
		async () => {
			const info = await api.request<{ status: string }>(
				'GET',
				`/workspace/${workspaceId}/deployment/${deploymentId}`,
				{ token: user.accessToken, clientIp: user.clientIp },
			);
			last = info.status;
			return targets.includes(info.status);
		},
		{
			timeoutMs: opts.timeoutMs ?? 120_000,
			intervalMs: 1000,
			label: `deployment ${deploymentId} → ${targets.join('|')}`,
		},
	);
	return last;
}

// The host of a deployment's default ingress URL (routed by Caddy). Under
// e2e_http_ingress this is served over plain HTTP on the runner's published
// ingress port.
export function deploymentDefaultUrlHost(deploymentId: string, port: number): string {
	return `${port}-${deploymentId}.onpatr.cloud`;
}
