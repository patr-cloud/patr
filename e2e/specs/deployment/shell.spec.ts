import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { waitForDeploymentStatus } from '@/helpers/deployment';
import { DeploymentShellStream } from '@/helpers/deployment-shell-stream';

// End-to-end interactive shell over a REAL docker runner: `patr deployment
// shell` drops into `/bin/sh` inside the running container via
// exec-with-a-TTY, and stdin/stdout are proxied through the API's Redis-List
// bridge. @docker → only runs in the docker-NN projects (DinD-backed).
//
// The CLI-facing backpressure gate is covered deterministically by the Rust
// integration test `shell_runner_backpressure_plateaus`; here we prove the real
// exec path end-to-end, including that a large flood arrives intact (backpressure
// throttles the container rather than dropping bytes).
//
// NOTE: a companion test that drives the built `patr` CLI binary under a
// pseudo-terminal (raw mode, Ctrl-] escape, exit-code propagation) needs a pty
// library (`node-pty`), which is not yet an e2e dependency — tracked as a
// follow-up.

function dockerVersionOf(testInfo: {
  project: { metadata: { dockerVersion?: string } };
}): DockerVersion {
  return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

test.beforeAll(async () => {
  await seedMachineType();
});

test.describe('deployment > shell @docker', () => {
  test('opens an interactive shell, echoes stdin, resizes, and exits', async ({
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

    // nginx: long-running and ships /bin/sh + coreutils (needed for the flood
    // and `stty size`).
    const repo = await createContainerRepo(api, user, user.workspaceId);
    await pushImageToPatrRegistry({
      dockerHost: runner.docker.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'latest',
      apiToken: runner.apiToken,
      sourceImage: 'nginx:latest',
    });

    const dep = await createDeploymentAPI(api, user, user.workspaceId, {
      repositoryId: repo.id,
      runnerId: runner.runnerId,
      imageTag: 'latest',
      port: 80,
      deployOnCreate: true,
    });
    const status = await waitForDeploymentStatus(api, user, user.workspaceId, dep.id, 'running', {
      timeoutMs: 180_000,
    });
    expect(status).toBe('running');

    await using shell = await DeploymentShellStream.open({
      workspaceId: user.workspaceId,
      deploymentId: dep.id,
      token: runner.apiToken,
      clientIp: user.clientIp,
    });

    // The runner dials back and execs; wait for the live-shell signal.
    await shell.next((m) => m.type === 'Connected', 30_000);

    // 1. Echo: stdin round-trips through the container.
    const marker = `hello-${crypto.randomUUID().slice(0, 8)}`;
    shell.sendStdin(`echo ${marker}\n`);
    await shell.waitForOutput(marker);

    // 2. Resize propagates to the PTY (stty reads the terminal size).
    shell.sendResize(30, 100);
    shell.sendStdin('stty size\n');
    await shell.waitForOutput('30 100');

    // 3. Large-output integrity: flood 20k lines of 'x'. Every byte must arrive
    // (backpressure throttles the container, never drops). We poll the received
    // 'x' count directly rather than a sentinel — the PTY echoes the typed
    // command, so a literal sentinel would match its own echo before the flood.
    shell.sendStdin(`yes x | head -n 20000\n`);
    const deadline = Date.now() + 60_000;
    const countXs = () => (shell.output.match(/x/g) ?? []).length;
    while (countXs() < 20_000 && Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 100));
    }
    expect(countXs()).toBeGreaterThanOrEqual(20_000);

    // 4. Clean exit propagates an Exit frame with the shell's code.
    shell.sendStdin('exit\n');
    const exit = await shell.next((m) => m.type === 'Exit', 30_000);
    expect(exit.type).toBe('Exit');
  });

  test('shelling into a stopped deployment fails loud', async ({ api }, testInfo) => {
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
      sourceImage: 'nginx:latest',
    });
    // Created but NOT deployed → stays stopped.
    const dep = await createDeploymentAPI(api, user, user.workspaceId, {
      repositoryId: repo.id,
      runnerId: runner.runnerId,
      imageTag: 'latest',
      port: 80,
      deployOnCreate: false,
    });

    await using shell = await DeploymentShellStream.open({
      workspaceId: user.workspaceId,
      deploymentId: dep.id,
      token: runner.apiToken,
      clientIp: user.clientIp,
    });
    const err = await shell.next((m) => m.type === 'Error', 15_000);
    expect(String(err.message)).toContain('not running');
  });
});
