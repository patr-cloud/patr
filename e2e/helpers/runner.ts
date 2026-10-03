import { ChildProcess, spawn } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import type { ApiClient } from '@/helpers/api';
import { createApiTokenAPI } from '@/helpers/api-token';
import { createRunnerAPI } from '@/helpers/runner-api';
import { DindHandle, type DockerVersion } from '@/helpers/dind';
import { FauxEdge } from '@/helpers/faux-edge';
import { onExit, waitFor } from '@/helpers/process';

const REPO_ROOT = resolve(import.meta.dirname, '..', '..');
const RUNNER_BINARY = join(REPO_ROOT, 'target', 'debug', 'docker');

// Pool of bind ports per worker. The runner's own HTTP listener; each parallel
// runner needs its own. We pick from an ephemeral-ish range above 24000.
let nextBindPort = 24000;
function allocateBindPort(): number {
	return nextBindPort++;
}

export type RunnerOpts = {
	api: ApiClient;
	// The workspace owner (or any member who can create a runner + mint an API
	// token). Used to provision the runner and its token.
	user: { accessToken: string; clientIp: string };
	// The workspace the runner belongs to. Deployments/repos under test should
	// live in the same workspace so the runner can serve them.
	workspaceId: string;
	dockerVersion: DockerVersion;
	name?: string;
};

export class RunnerHandle implements AsyncDisposable {
	private constructor(
		private readonly proc: ChildProcess,
		private readonly dind: DindHandle,
		private readonly fauxEdge: FauxEdge,
		public readonly runnerId: string,
		public readonly workspaceId: string,
		public readonly apiToken: string,
		public readonly bindPort: number,
	) {}

	// The DinD daemon backing this runner — exposes dockerHost (for push/build)
	// and hitIngress (to curl running deployments through Caddy).
	get docker(): DindHandle {
		return this.dind;
	}

	static async connect(opts: RunnerOpts): Promise<RunnerHandle> {
		const dind = await DindHandle.spawn(opts.dockerVersion);

		try {
			const { runnerId, serviceAccountToken, apiToken } = await provisionRunner(opts);

			const bindPort = allocateBindPort();
			const workDir = mkdtempSync(join(tmpdir(), 'patr-e2e-runner-'));
			const dbPath = join(workDir, 'runner.db');

			const proc = spawn(RUNNER_BINARY, [], {
				cwd: REPO_ROOT,
				env: {
					...process.env,
					DOCKER_HOST: dind.dockerHost,
					// Config keys use the `PATR__` prefix with `__` as the nesting
					// separator; segment names are converted to camelCase to match the
					// serde field names (see runners/common/src/utils/config.rs).
					PATR__MODE: 'managed',
					PATR__WORKSPACE_ID: opts.workspaceId,
					PATR__RUNNER_ID: runnerId,
					// The runner authenticates as its own service account, exactly
					// as a real `patr runner setup` leaves it configured.
					PATR__API_TOKEN: serviceAccountToken,
					PATR__BIND_ADDRESS: `127.0.0.1:${bindPort}`,
					PATR__DATABASE__FILE: dbPath,
					PATR__DATABASE__CONNECTION_LIMIT: '5',
					// The runner pulls from the hardcoded registry.patr.cloud, which the
					// DinD resolves to the host registry via a socat bridge (see dind.ts).
					// Publish the Caddy ingress on 8080 (not 80) so the swarm routing mesh
					// leaves :80/:443 free for the in-DinD registry bridge (see dind.ts).
					PATR__INGRESS_HTTP_LISTEN_PORT: '8080',
					// A DinD swarm has no IPv6 address pool, so an IPv6-enabled overlay
					// rejects every task; disable it on the deployment overlay here.
					PATR__ENABLE_IPV6: 'false',
				},
				// Inherit stdout/stderr so the runner's logs stream straight to the test
				// output (and CI), never to a file.
				stdio: ['ignore', 'inherit', 'inherit'],
			});

			proc.once('exit', (code) => {
				if (code !== null && code !== 0) {
					console.error(`runner ${runnerId} exited early with code ${code}`);
				}
			});

			try {
				await waitFor(
					async () => isRunnerConnected(opts.api, opts.user, opts.workspaceId, runnerId),
					{
						timeoutMs: 60_000,
						label: `runner ${runnerId} connected`,
					},
				);
			} catch (err) {
				proc.kill('SIGKILL');
				throw err;
			}

			// Stand up the faux edge (TLS terminator) in front of the ingress so tests
			// reach deployments over the production topology, not Caddy directly.
			let fauxEdge: FauxEdge;
			try {
				fauxEdge = await FauxEdge.deploy(dind);
			} catch (err) {
				proc.kill('SIGKILL');
				throw err;
			}

			return new RunnerHandle(
				proc,
				dind,
				fauxEdge,
				runnerId,
				opts.workspaceId,
				apiToken,
				bindPort,
			);
		} catch (err) {
			await dind[Symbol.asyncDispose]();
			throw err;
		}
	}

	async [Symbol.asyncDispose](): Promise<void> {
		this.proc.kill('SIGTERM');
		await onExit(this.proc);
		await this.fauxEdge[Symbol.asyncDispose]();
		await this.dind[Symbol.asyncDispose]();
	}
}

async function provisionRunner(
	opts: RunnerOpts,
): Promise<{ runnerId: string; serviceAccountToken: string; apiToken: string }> {
	const name = opts.name ?? `e2e-runner-${crypto.randomUUID().slice(0, 8)}`;
	// Go through the consent-link flow, which is the only way to mint a runner
	// now. It hands back the runner's own service account token — the same
	// credential a real `patr runner setup` writes to disk. Approve grants it
	// "Runner: All Resource Reader" across the workspace plus "Runner: Execute"
	// on this runner, which covers opening the stream and fetching images.
	const runner = await createRunnerAPI(opts.api, opts.user, opts.workspaceId, name);

	// Pushing an image is a developer/CI action, not something the runner does —
	// and the runner's service account deliberately has pull but not push. So
	// tests that push need a separate workspace-wide user token.
	const pushToken = await createApiTokenAPI(opts.api, opts.user, {
		superAdminOf: [opts.workspaceId],
	});

	return {
		runnerId: runner.id,
		serviceAccountToken: runner.token,
		apiToken: pushToken.token,
	};
}

export async function isRunnerConnected(
	api: ApiClient,
	user: { accessToken: string; clientIp: string },
	workspaceId: string,
	runnerId: string,
): Promise<boolean> {
	const info = await api.request<{ runner: { connected: boolean } }>(
		'GET',
		`/workspace/${workspaceId}/runner/${runnerId}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return info.runner.connected === true;
}
