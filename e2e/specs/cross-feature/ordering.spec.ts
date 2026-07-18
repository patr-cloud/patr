import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { createDeploymentAPI, stopDeploymentAPI } from '@/helpers/deployment-api';
import { waitForDeploymentStatus, deploymentDefaultUrlHost } from '@/helpers/deployment';
import {
	createVerifiedDomain,
	createManagedUrlAPI,
	proxyDeploymentBody,
	randomSubdomain,
} from '@/helpers/managed-url-api';

// Cross-feature ordering: behaviours that depend on the order operations happen
// in, exercised against a real runner.

function dockerVersionOf(testInfo: {
	project: { metadata: { dockerVersion?: string } };
}): DockerVersion {
	return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

async function hitUntil(
	runner: RunnerHandle,
	host: string,
	predicate: (res: IngressResponse) => boolean,
	attempts = 30,
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

test.beforeAll(async () => {
	await seedMachineType();
});

test.describe('cross-feature > ordering @docker', () => {
	test('two deployments on one runner each serve on their own ingress host', async ({
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

		const a = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: true,
		});
		const b = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: true,
		});

		await waitForDeploymentStatus(api, user, user.workspaceId, a.id, 'running', {
			timeoutMs: 180_000,
		});
		await waitForDeploymentStatus(api, user, user.workspaceId, b.id, 'running', {
			timeoutMs: 180_000,
		});

		// Each deployment is reachable on its own distinct ingress host.
		const resA = await hitUntil(
			runner,
			deploymentDefaultUrlHost(a.id, 80),
			(r) => r.status === 200,
		);
		const resB = await hitUntil(
			runner,
			deploymentDefaultUrlHost(b.id, 80),
			(r) => r.status === 200,
		);
		expect(resA?.status).toBe(200);
		expect(resB?.status).toBe(200);
		expect(resA!.body).toContain('Hostname');
		expect(resB!.body).toContain('Hostname');
	});

	test('a managed URL serves while its deployment runs and breaks when it is stopped', async ({
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
			deployOnCreate: true,
		});
		await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'running', {
			timeoutMs: 180_000,
		});

		const domain = await createVerifiedDomain(api, user, user.workspaceId);
		const sub = randomSubdomain();
		await createManagedUrlAPI(
			api,
			user,
			user.workspaceId,
			proxyDeploymentBody({
				domainId: domain.id,
				deploymentId: dep.id,
				port: 80,
				subDomain: sub,
			}),
		);
		const host = `${sub}.${domain.domain}`;

		// Serves while the deployment runs.
		const up = await hitUntil(runner, host, (r) => r.status === 200);
		expect(up?.status).toBe(200);
		expect(up!.body).toContain('Hostname');

		// Stopping the deployment tears the service down → the managed URL breaks.
		await stopDeploymentAPI(api, user, user.workspaceId, dep.id);
		await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'stopped', {
			timeoutMs: 120_000,
		});
		const down = await hitUntil(runner, host, (r) => r.status !== 200, 15);
		expect(down === undefined || down.status !== 200).toBe(true);
	});
});
