import { execa } from 'execa';
import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { waitForDeploymentStatus, deploymentDefaultUrlHost } from '@/helpers/deployment';
import { createSecretAPI, randomSecretName, updateSecretAPI } from '@/helpers/secret-api';

// Rotation end to end: a real docker runner resolves a secret env var through
// the API's OpenBao proxy, and a rotated value rolls the service without a
// redeploy, while a rename leaves it alone. The version bookkeeping behind this
// is unit-tested in runners/common/tests/managed_mode.rs.

// Hit a deployment through the runner's Caddy ingress, retrying while the swarm
// mesh / Caddy reload settles. Returns the last response (or undefined).
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

// The container running the deployment's swarm task. A rolled service replaces
// its task, and with it the container.
async function taskContainerId(runner: RunnerHandle, deploymentId: string): Promise<string> {
	const { stdout } = await execa('docker', [
		'-H',
		runner.docker.dockerHost,
		'ps',
		'-q',
		'--filter',
		`label=patr.deploymentId=${deploymentId}`,
	]);
	return stdout.trim();
}

function dockerVersionOf(testInfo: {
	project: { metadata: { dockerVersion?: string } };
}): DockerVersion {
	return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

test.beforeAll(async () => {
	await seedMachineType();
});

test.describe('secret > rotation @docker', () => {
	test('a rotated secret rolls the service; a rename does not', async ({ api }, testInfo) => {
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

		// traefik/whoami echoes WHOAMI_NAME as a "Name: <value>" line.
		const first = `e2e-${crypto.randomUUID().slice(0, 8)}`;
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, first);
		const dep = await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.runnerId,
			imageTag: 'latest',
			port: 80,
			deployOnCreate: true,
			environmentVariables: { WHOAMI_NAME: { fromSecret: secret.id } },
		});

		expect(
			await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'running', {
				timeoutMs: 180_000,
			}),
		).toBe('running');

		const host = deploymentDefaultUrlHost(dep.id, 80);
		const initial = await hitUntil(runner, host, (r) => r.body.includes(`Name: ${first}`));
		expect(initial?.body).toContain(`Name: ${first}`);
		const initialContainer = await taskContainerId(runner, dep.id);
		expect(initialContainer).not.toBe('');

		// A rename doesn't touch the value, so the service must keep its task.
		await updateSecretAPI(api, user, user.workspaceId, secret.id, {
			name: randomSecretName('RENAMED'),
		});
		await new Promise((r) => setTimeout(r, 15_000));
		expect(await taskContainerId(runner, dep.id)).toBe(initialContainer);

		// A rotation rolls the service onto the new value, with no redeploy.
		const second = `e2e-${crypto.randomUUID().slice(0, 8)}`;
		await updateSecretAPI(api, user, user.workspaceId, secret.id, {
			name,
			value: second,
		});
		const rotated = await hitUntil(runner, host, (r) => r.body.includes(`Name: ${second}`));
		expect(rotated?.body).toContain(`Name: ${second}`);
		expect(await taskContainerId(runner, dep.id)).not.toBe(initialContainer);
	});
});
