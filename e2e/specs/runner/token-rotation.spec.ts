import { execa } from 'execa';
import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { waitForDeploymentStatus, deploymentDefaultUrlHost } from '@/helpers/deployment';
import { waitFor } from '@/helpers/process';
import { isRunnerConnected } from '@/helpers/runner';
import { regenerateRunnerTokenAPI } from '@/helpers/runner-api';

// Moving a runner to a new token, the way `patr runner setup reconnect` does
// on a live box: the old token is cut off, the runner restarts on the same
// database with the new one, and the deployment it runs never redeploys.

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

// The container running the deployment's swarm task. A redeploy replaces the
// task, and with it the container.
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

test.describe('runner > token rotation @docker', () => {
	test('a regenerated token disconnects the runner, and restarting with it keeps the same task serving', async ({
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
		expect(
			await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'running', {
				timeoutMs: 180_000,
			}),
		).toBe('running');

		const host = deploymentDefaultUrlHost(dep.id, 80);
		const initial = await hitUntil(runner, host, (r) => r.body.includes(`Name: ${marker}`));
		expect(initial?.body).toContain(`Name: ${marker}`);
		const initialContainer = await taskContainerId(runner, dep.id);
		expect(initialContainer).not.toBe('');

		// The old connection is closed at its next ping, within ~30s.
		const newToken = await regenerateRunnerTokenAPI(
			api,
			user,
			user.workspaceId,
			runner.runnerId,
		);
		await waitFor(
			async () => !(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)),
			{ timeoutMs: 60_000, intervalMs: 1000, label: 'runner disconnected after rotation' },
		);

		// Offline, the deployment keeps serving from the same task.
		const offline = await hitUntil(runner, host, (r) => r.body.includes(`Name: ${marker}`));
		expect(offline?.body).toContain(`Name: ${marker}`);
		expect(await taskContainerId(runner, dep.id)).toBe(initialContainer);

		// Back online on the same database with the new token, and nothing redeploys
		// once the resync has settled.
		await runner.restart({ token: newToken });
		await new Promise((r) => setTimeout(r, 15_000));
		expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(true);
		expect(await taskContainerId(runner, dep.id)).toBe(initialContainer);
		const after = await hitUntil(runner, host, (r) => r.body.includes(`Name: ${marker}`));
		expect(after?.body).toContain(`Name: ${marker}`);
	});
});
