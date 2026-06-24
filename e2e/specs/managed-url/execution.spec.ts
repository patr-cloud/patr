import { test, expect, createUserWithWorkspace, RunnerHandle } from '@/prelude';
import type { DockerVersion } from '@/prelude';
import type { IngressResponse } from '@/helpers/dind';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo, pushImageToPatrRegistry } from '@/helpers/registry';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import { waitForDeploymentStatus } from '@/helpers/deployment';
import {
  createVerifiedDomain,
  createManagedUrlAPI,
  proxyDeploymentBody,
  randomSubdomain,
} from '@/helpers/managed-url-api';

// Full-fidelity: a ProxyDeployment managed URL is served by the runner's Caddy
// ingress, routed by the managed-URL Host (`{sub}.{domain}`). Only
// ProxyDeployment is runner-served; ProxyUrl/Redirect/StaticSite are
// Cloudflare-Worker-only and out of @docker scope.

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

test.describe('managed-url > execution @docker', () => {
  test('a ProxyDeployment managed URL is served through the ingress', async ({ api }, testInfo) => {
    test.setTimeout(300_000);
    await using user = await createUserWithWorkspace(api);
    await using runner = await RunnerHandle.connect({
      api,
      user,
      workspaceId: user.workspaceId,
      dockerVersion: dockerVersionOf(testInfo),
    });

    // A running deployment to proxy to.
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

    // A managed URL on a verified domain pointing at the deployment.
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
        path: '/',
      }),
    );

    // The runner reconfigures Caddy for the managed-URL host; hit it.
    const host = `${sub}.${domain.domain}`;
    const res = await hitUntil(runner, host, (r) => r.status === 200);
    expect(res?.status).toBe(200);
    expect(res!.body).toContain('Hostname');
  });
});
