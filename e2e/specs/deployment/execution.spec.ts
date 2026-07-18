import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { waitFor } from '@/helpers/process';
import {
	createDeploymentAPI,
	getDeploymentInfoAPI,
	startDeploymentAPI,
	stopDeploymentAPI,
} from '@/helpers/deployment-api';
import { waitForDeploymentStatus, deploymentDefaultUrlHost } from '@/helpers/deployment';

// Full-fidelity execution: a real docker runner pulls the image, runs it on a
// DinD swarm, and the deployment is reachable through the Caddy ingress. These
// are the @docker tests (DinD-backed), so they only run in the docker-NN
// projects. Each spins one runner; keep the count lean — DinD setup dominates.

// Hit a deployment through the runner's Caddy ingress, retrying while the swarm
// mesh / Caddy reload settles. Returns the last response (or undefined).
async function hitUntil(
	runner: RunnerHandle,
	host: string,
	predicate: (res: IngressResponse) => boolean,
	attempts = 20,
): Promise<IngressResponse | undefined> {
	let res: IngressResponse | undefined;
	for (let i = 0; i < attempts; i++) {
		try {
			res = await runner.docker.hitIngress(host);
			if (predicate(res)) return res;
		} catch {
			res = undefined;
		}
		await new Promise((r) => setTimeout(r, 2000));
	}
	return res;
}

function dockerVersionOf(testInfo: {
	project: { metadata: { dockerVersion?: string } };
}): DockerVersion {
	return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

test.beforeAll(async () => {
	await seedMachineType();
});

test.describe('deployment > execution @docker', () => {
	test('a Patr-image deployment runs and exposes its env var through the ingress', async ({
		api,
	}, testInfo) => {
		test.setTimeout(240_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dockerVersionOf(testInfo),
		});

		const repo = await createContainerRepo(api, user, user.workspaceId);
		await pushImageToPatrRegistry({
			dockerHost: runner.docker.dockerHost,
			workspaceId: user.workspaceId,
			repoName: repo.name,
			tag: 'latest',
			apiToken: runner.apiToken,
		});

		// traefik/whoami echoes WHOAMI_NAME as a "Name: <value>" line.
		const marker = `e2e-${crypto.randomUUID().slice(0, 8)}`;
		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: true,
			environmentVariables: { WHOAMI_NAME: marker },
		});

		const status = await waitForDeploymentStatus(
			api,
			user,
			user.workspaceId,
			dep.id,
			'running',
			{
				timeoutMs: 180_000,
			},
		);
		expect(status).toBe('running');

		const host = deploymentDefaultUrlHost(dep.id, 80);
		const res = await hitUntil(runner, host, (r) => r.status === 200);
		expect(res?.status).toBe(200);
		expect(res!.body).toContain('Hostname');
		// The env var reached the container.
		expect(res!.body).toContain(marker);
	});

	test('an external (docker.io) image deployment runs and serves', async ({ api }, testInfo) => {
		test.setTimeout(240_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dockerVersionOf(testInfo),
		});

		// No registry push — the runner pulls docker.io/traefik/whoami directly.
		const created = await api.request<{ id: string }>(
			'POST',
			`/workspace/${user.workspaceId}/deployment`,
			{
				token: user.accessToken,
				clientIp: user.clientIp,
				body: {
					name: `ext-${crypto.randomUUID().slice(0, 8)}`,
					registry: 'docker.io',
					imageName: 'traefik/whoami',
					imageTag: 'latest',
					runner: runner.runnerId,
					machineType: 'b3cf3771fa394281bfdfeb2e65a061b6',
					deployOnPush: false,
					minHorizontalScale: 1,
					maxHorizontalScale: 1,
					ports: { '80': 'http' },
					deployOnCreate: true,
				},
			},
		);

		const status = await waitForDeploymentStatus(
			api,
			user,
			user.workspaceId,
			created.id,
			'running',
			{
				timeoutMs: 180_000,
			},
		);
		expect(status).toBe('running');

		const host = deploymentDefaultUrlHost(created.id, 80);
		const res = await hitUntil(runner, host, (r) => r.status === 200);
		expect(res?.status).toBe(200);
		expect(res!.body).toContain('Hostname');
	});

	test('deployOnCreate=false stays stopped until started; stop tears the ingress down', async ({
		api,
	}, testInfo) => {
		test.setTimeout(300_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dockerVersionOf(testInfo),
		});

		const repo = await createContainerRepo(api, user, user.workspaceId);
		await pushImageToPatrRegistry({
			dockerHost: runner.docker.dockerHost,
			workspaceId: user.workspaceId,
			repoName: repo.name,
			tag: 'latest',
			apiToken: runner.apiToken,
		});

		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: false,
		});
		// Created stopped — the runner never starts it.
		expect((await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id)).status).toBe(
			'stopped',
		);

		// Start it → reaches Running and serves.
		await startDeploymentAPI(api, user, user.workspaceId, dep.id);
		expect(
			await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'running', {
				timeoutMs: 180_000,
			}),
		).toBe('running');
		const host = deploymentDefaultUrlHost(dep.id, 80);
		expect((await hitUntil(runner, host, (r) => r.status === 200))?.status).toBe(200);

		// Stop it → the swarm service is removed and the ingress stops serving.
		await stopDeploymentAPI(api, user, user.workspaceId, dep.id);
		await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'stopped', {
			timeoutMs: 120_000,
		});
		const down = await hitUntil(runner, host, (r) => r.status !== 200, 15);
		expect(down === undefined || down.status !== 200).toBe(true);
	});

	test('deploy-on-push redeploys a running deployment and skips a stopped one', async ({
		api,
	}, testInfo) => {
		test.setTimeout(360_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dockerVersionOf(testInfo),
		});

		const repo = await createContainerRepo(api, user, user.workspaceId);
		// Initial image (whoami) on :latest.
		await pushImageToPatrRegistry({
			dockerHost: runner.docker.dockerHost,
			workspaceId: user.workspaceId,
			repoName: repo.name,
			tag: 'latest',
			apiToken: runner.apiToken,
			sourceImage: 'traefik/whoami:latest',
		});

		// A running deploy-on-push deployment (will redeploy) and a stopped one (skip).
		const running = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: false,
			deployOnPush: true,
		});
		const stopped = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: false,
			deployOnPush: true,
		});

		// Start (not deployOnCreate) so current_live_digest is seeded synchronously
		// from the tag — deployOnCreate sets Deploying but never writes the digest.
		await startDeploymentAPI(api, user, user.workspaceId, running.id);
		await waitForDeploymentStatus(api, user, user.workspaceId, running.id, 'running', {
			timeoutMs: 180_000,
		});
		const firstDigest = (await getDeploymentInfoAPI(api, user, user.workspaceId, running.id))
			.currentLiveDigest;
		expect(firstDigest).toBeTruthy();

		// Push a DIFFERENT image to the same repo:tag → new digest → deploy-on-push.
		await pushImageToPatrRegistry({
			dockerHost: runner.docker.dockerHost,
			workspaceId: user.workspaceId,
			repoName: repo.name,
			tag: 'latest',
			apiToken: runner.apiToken,
			sourceImage: 'nginxdemos/hello:latest',
		});

		// The running deployment's live digest changes (auto-redeploy fired).
		await waitFor(
			async () => {
				const d = (await getDeploymentInfoAPI(api, user, user.workspaceId, running.id))
					.currentLiveDigest;
				return d !== null && d !== firstDigest;
			},
			{ timeoutMs: 120_000, intervalMs: 2000, label: 'deploy-on-push updates live digest' },
		);

		// The stopped deployment was skipped: no digest, still stopped.
		const stoppedInfo = await getDeploymentInfoAPI(api, user, user.workspaceId, stopped.id);
		expect(stoppedInfo.status).toBe('stopped');
		expect(stoppedInfo.currentLiveDigest).toBeNull();
	});

	test('a running deployment is reachable through the TLS edge without a redirect loop', async ({
		api,
	}, testInfo) => {
		test.setTimeout(240_000);
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion: dockerVersionOf(testInfo),
		});

		const created = await api.request<{ id: string }>(
			'POST',
			`/workspace/${user.workspaceId}/deployment`,
			{
				token: user.accessToken,
				clientIp: user.clientIp,
				body: {
					name: `loop-${crypto.randomUUID().slice(0, 8)}`,
					registry: 'docker.io',
					imageName: 'traefik/whoami',
					imageTag: 'latest',
					runner: runner.runnerId,
					machineType: 'b3cf3771fa394281bfdfeb2e65a061b6',
					deployOnPush: false,
					minHorizontalScale: 1,
					maxHorizontalScale: 1,
					ports: { '80': 'http' },
					deployOnCreate: true,
				},
			},
		);

		expect(
			await waitForDeploymentStatus(api, user, user.workspaceId, created.id, 'running', {
				timeoutMs: 180_000,
			}),
		).toBe('running');

		const host = deploymentDefaultUrlHost(created.id, 80);
		// Confirm it serves at all first (single hop, no redirect-follow).
		expect((await hitUntil(runner, host, (r) => r.status === 200))?.status).toBe(200);

		// Now follow redirects back through the faux edge. A correct private-runner
		// config serves plain HTTP behind the TLS-terminating edge, so this resolves
		// to 200 in one hop. If a regression made the ingress emit an HTTP->HTTPS
		// redirect, the edge would re-terminate TLS and forward HTTP again, bouncing
		// forever — hitIngress throws once it passes the cap, failing this test.
		const followed = await runner.docker.hitIngress(host, { maxRedirects: 5 });
		expect(followed.status).toBe(200);
		expect(followed.body).toContain('Hostname');
	});
});
