import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { isRunnerConnected } from '@/helpers/runner';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import {
	createPatrDeployment,
	waitForDeploymentStatus,
	deploymentDefaultUrlHost,
} from '@/helpers/deployment';

// Harness sanity check for the @docker runner stack. The foundation every
// full-fidelity registry/deployment/managed-url test builds on:
//   1. a real docker runner provisions + connects over the websocket;
//   2. an image pushed to the Patr registry is pulled and run on the runner,
//      and the running deployment is reachable through the Caddy ingress.
test.describe('@docker harness smoke', () => {
	test('runner provisions and connects to the API', async ({ api }, testInfo) => {
		test.setTimeout(120_000);
		const dockerVersion = (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;

		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion,
		});

		expect(runner.runnerId).toBeTruthy();

		// The DinD daemon is reachable on its published port.
		const ping = await fetch(`http://127.0.0.1:${runner.docker.hostPort}/_ping`);
		expect(ping.ok).toBe(true);

		// The API agrees the runner is connected.
		expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(true);
	});

	test('push image → deploy → reaches Running → serves through ingress', async ({
		api,
	}, testInfo) => {
		test.setTimeout(240_000);
		const dockerVersion = (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;

		await seedMachineType();
		await using user = await createUserWithWorkspace(api);
		await using runner = await RunnerHandle.connect({
			api,
			user,
			workspaceId: user.workspaceId,
			dockerVersion,
		});

		// Push an echo image into the Patr registry, then deploy it.
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await pushImageToPatrRegistry({
			dockerHost: runner.docker.dockerHost,
			workspaceId: user.workspaceId,
			repoName: repo.name,
			tag: 'latest',
			apiToken: runner.apiToken,
		});

		const deployment = await createPatrDeployment(api, user, {
			workspaceId: user.workspaceId,
			repositoryId: repo.id,
			imageTag: 'latest',
			runnerId: runner.runnerId,
			port: 80,
			deployOnCreate: true,
		});

		// The runner pulls the image from the registry and starts the swarm service.
		const status = await waitForDeploymentStatus(
			api,
			user,
			user.workspaceId,
			deployment.id,
			'running',
			{ timeoutMs: 180_000 },
		);
		expect(status).toBe('running');

		// Hit it through the Caddy ingress and confirm the container responds.
		// Right after the task starts, the swarm mesh / Caddy reload can briefly
		// reset or 404, so retry tolerating both exceptions and non-200.
		const host = deploymentDefaultUrlHost(deployment.id, deployment.port);
		let res: IngressResponse | undefined;
		for (let i = 0; i < 20; i++) {
			try {
				res = await runner.docker.hitIngress(host);
				if (res.status === 200) break;
			} catch {
				res = undefined;
			}
			await new Promise((r) => setTimeout(r, 2000));
		}
		expect(res?.status).toBe(200);
		// traefik/whoami echoes request info incl. the Host it received.
		expect(res!.body).toContain('Hostname');
	});
});
