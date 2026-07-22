import { request as httpsRequest } from 'node:https';
import { execa } from 'execa';
import { HOST_API_PORT, HOST_REGISTRY_PORT, REGISTRY_HOST } from '@/helpers/urls';
import { waitFor } from '@/helpers/process';

export type IngressResponse = {
	status: number;
	body: string;
	headers: Record<string, string | string[] | undefined>;
};

export type DockerVersion = '24' | '25' | '26';

// A Docker-in-Docker daemon for the @docker suite. Each handle is one
// privileged `docker:NN-dind` container running its own dockerd, reachable from
// the host on an ephemeral port. The runner binary points at it via DOCKER_HOST.
//
// Network wiring (verified against OrbStack):
// - `--add-host=host.docker.internal:host-gateway` makes the real host
//   reachable from inside the DinD container (and thus from the dockerd that
//   pulls images and from the swarm tasks it schedules).
// - `--add-host=registry.patr.cloud:127.0.0.1` makes the runner's hardcoded
//   registry host resolve to the DinD itself, where a socat bridge (set up in
//   `setupRegistryBridge`) forwards :80 and :443 → host registry (:3002) and
//   :3000 → host API (:3000, the docker-login token realm). This lets the runner
//   pull from `registry.patr.cloud` with no production code override; pushes go
//   through the same daemon. `--insecure-registry registry.patr.cloud` lets
//   dockerd use plain HTTP for the registry.
// - port 2375 (dockerd API) and port 8080 (the Caddy ingress — the runner
//   publishes it on 8080 not 80 so the swarm routing mesh doesn't shadow the
//   registry bridge on :80/:443) are each published to an ephemeral host port so
//   the test can drive the daemon and curl running deployments.
export class DindHandle implements AsyncDisposable {
	private constructor(
		public readonly containerId: string,
		public readonly hostPort: number,
		public readonly ingressPort: number,
		public readonly version: DockerVersion,
	) {}

	static async spawn(version: DockerVersion): Promise<DindHandle> {
		const name = `e2e-dind-${crypto.randomUUID().slice(0, 8)}`;

		const { stdout: cid } = await execa('docker', [
			'run',
			'--rm',
			'-d',
			'--privileged',
			'--name',
			name,
			// Reach the host (API + registry) from inside DinD.
			'--add-host=host.docker.internal:host-gateway',
			// The runner's hardcoded registry host resolves to the DinD's own socat bridge.
			'--add-host=registry.patr.cloud:127.0.0.1',
			// dockerd API + the swarm ingress (published on 8080), each on a random host port.
			'-p',
			'0:2375',
			'-p',
			'0:8080',
			`docker:${version}-dind`,
			'dockerd',
			'--host=tcp://0.0.0.0:2375',
			`--insecure-registry=${REGISTRY_HOST}`,
		]);

		const containerId = cid.trim();

		let hostPort: number;
		let ingressPort: number;
		try {
			hostPort = await resolvePort(containerId, '2375/tcp');
			ingressPort = await resolvePort(containerId, '8080/tcp');

			await waitFor(
				async () => {
					const res = await fetch(`http://127.0.0.1:${hostPort}/_ping`);
					return res.ok;
				},
				{ timeoutMs: 30_000, label: `dind ${version} ready` },
			);

			await setupRegistryBridge(containerId, version);
		} catch (err) {
			await execa('docker', ['rm', '-f', containerId]).catch(() => {});
			throw err;
		}

		return new DindHandle(containerId, hostPort, ingressPort, version);
	}

	get dockerHost(): string {
		return `tcp://127.0.0.1:${this.hostPort}`;
	}

	// Hit a deployment/managed URL through the faux edge → Caddy ingress. Routing
	// is by Host header (e.g. `{port}-{deploymentId}.onpatr.cloud` for the default
	// deployment URL, or `{sub}.{domain}{path}` for a managed URL). Goes over HTTPS
	// because the faux edge terminates TLS in front of the ingress, exactly like
	// Cloudflare does in production (the cert is self-signed, hence
	// `rejectUnauthorized: false`). Uses node:https (not fetch) because `Host` is a
	// forbidden header in the Fetch API — undici silently drops it, so Caddy would
	// never see the intended virtual host.
	//
	// By default it does not follow redirects, so callers can assert 301/302 from
	// redirect-type managed URLs. Pass `maxRedirects` to follow them back through
	// the edge — used to catch TLS-offload redirect loops, which throw once the cap
	// is exceeded instead of hanging.
	async hitIngress(
		host: string,
		opts: { path?: string; method?: string; maxRedirects?: number } = {},
	): Promise<IngressResponse> {
		const maxRedirects = opts.maxRedirects ?? 0;
		let currentHost = host;
		let currentPath = opts.path ?? '/';

		for (let hop = 0; ; hop++) {
			const res = await this.requestIngress(currentHost, currentPath, opts.method ?? 'GET');
			const location = res.headers.location;
			const isRedirect =
				res.status >= 300 && res.status < 400 && typeof location === 'string';

			if (!isRedirect || maxRedirects === 0) return res;
			if (hop >= maxRedirects) {
				throw new Error(
					`redirect loop hitting ${host}: exceeded ${maxRedirects} hops (last → ${location})`,
				);
			}

			// Re-issue the redirect target back through the edge: keep pointing at the
			// faux edge but swap in the redirected Host/path so the loop can form.
			const next = new URL(location, `https://${currentHost}${currentPath}`);
			currentHost = next.host;
			currentPath = `${next.pathname}${next.search}`;
		}
	}

	private requestIngress(host: string, path: string, method: string): Promise<IngressResponse> {
		return new Promise((resolve, reject) => {
			const req = httpsRequest(
				{
					host: '127.0.0.1',
					port: this.ingressPort,
					path,
					method,
					headers: { Host: host },
					servername: host,
					rejectUnauthorized: false,
				},
				(res) => {
					const chunks: Buffer[] = [];
					res.on('data', (c) => chunks.push(c));
					res.on('end', () =>
						resolve({
							status: res.statusCode ?? 0,
							body: Buffer.concat(chunks).toString('utf8'),
							headers: res.headers,
						}),
					);
				},
			);
			req.on('error', reject);
			req.end();
		});
	}

	// Read a file from inside the running swarm task of a deployment. Locates the
	// task container by the `patr.deploymentId` label the runner stamps on every
	// container spec, then `cat`s the target path against the DinD's own dockerd.
	// Used to assert the exact bytes of a mounted config (the double-base64
	// regression left base64 text in the file). Throws if no task is running yet,
	// so callers can poll it through `waitFor`.
	async readDeploymentFile(deploymentId: string, path: string): Promise<string> {
		const { stdout: ids } = await execa('docker', [
			'-H',
			this.dockerHost,
			'ps',
			'-q',
			'--filter',
			`label=patr.deploymentId=${deploymentId}`,
		]);
		const containerId = ids.trim().split('\n').filter(Boolean)[0];
		if (!containerId) {
			throw new Error(`no running container for deployment ${deploymentId}`);
		}
		const { stdout } = await execa('docker', [
			'-H',
			this.dockerHost,
			'exec',
			containerId,
			'cat',
			path,
		]);
		return stdout;
	}

	async [Symbol.asyncDispose](): Promise<void> {
		await execa('docker', ['rm', '-f', this.containerId]).catch(() => {});
	}
}

async function resolvePort(containerId: string, containerPort: string): Promise<number> {
	const { stdout } = await execa('docker', ['port', containerId, containerPort]);
	const match = stdout.match(/:(\d+)\s*$/m);
	if (!match) throw new Error(`could not parse ${containerPort} mapping: ${stdout}`);
	return Number(match[1]);
}

// Install socat in the DinD and start TCP bridges so `registry.patr.cloud`
// (→ 127.0.0.1 via --add-host) reaches the host API:
//   - :80 and :443 → host registry (the OCI listener at API port + 2). dockerd
//     talking to a port-less insecure registry tries HTTPS on :443 and HTTP on
//     :80, so both must bridge to the plain-HTTP registry.
//   - :3000 → host API (the docker-login token realm, http://localhost:3000).
// These ports are free inside the DinD because the swarm ingress is published on
// 8080, not 80/443.
async function setupRegistryBridge(containerId: string, version: DockerVersion): Promise<void> {
	await execa('docker', ['exec', containerId, 'apk', 'add', '--no-cache', 'socat']);
	const bridge = (listen: number, targetPort: number) =>
		execa('docker', [
			'exec',
			'-d',
			containerId,
			'socat',
			`TCP-LISTEN:${listen},fork,reuseaddr`,
			`TCP:host.docker.internal:${targetPort}`,
		]);
	await bridge(80, HOST_REGISTRY_PORT);
	await bridge(443, HOST_REGISTRY_PORT);
	await bridge(HOST_API_PORT, HOST_API_PORT);

	// Wait until the registry answers (401 over the bridge means it's reachable).
	await waitFor(
		async () => {
			const res = await execa(
				'docker',
				[
					'exec',
					containerId,
					'wget',
					'-q',
					'-O',
					'/dev/null',
					'-T',
					'3',
					`http://${REGISTRY_HOST}/v2/`,
				],
				{ reject: false },
			);
			return /\b401\b|Unauthorized/i.test(res.stderr) || res.exitCode === 0;
		},
		{ timeoutMs: 30_000, label: `dind ${version} registry bridge` },
	);
}
