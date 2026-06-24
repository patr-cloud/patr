import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  loginAs,
  getPermissionId,
  createApiTokenAPI,
  DindHandle,
} from '@/prelude';
import type { ApiClient, UserHandle, DockerVersion } from '@/prelude';
import {
  createContainerRepo,
  pushImageToPatrRegistry,
  tryPushImage,
  pullImageFromPatrRegistry,
  dockerLoginPatr,
  listManifestsAPI,
  listTagsAPI,
  exposedPortsAPI,
  randomRepoName,
} from '@/helpers/registry';
import { openRegistryDetail, imagesTab } from '@/helpers/ui/container-registry';

const SECOND_IMAGE = 'busybox:latest';

type User = UserHandle & { workspaceId: string };

function dv(testInfo: { project: { metadata: Record<string, unknown> } }): DockerVersion {
  return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

async function superAdminToken(api: ApiClient, user: User): Promise<string> {
  const t = await createApiTokenAPI(api, user, {
    permissions: { [user.workspaceId]: { type: 'superAdmin' } },
  });
  return t.token;
}

async function scopedToken(api: ApiClient, user: User, permName: string): Promise<string> {
  const id = await getPermissionId(
    api,
    user.accessToken,
    user.workspaceId,
    user.clientIp,
    permName,
  );
  const t = await createApiTokenAPI(api, user, {
    permissions: {
      [user.workspaceId]: {
        type: 'member',
        [id]: { permissionType: 'exclude', resources: [] },
      } as any,
    },
  });
  return t.token;
}

test.describe('@docker container registry push/pull', () => {
  test('push → manifest + tag recorded → pull back → exposed ports', async ({ api }, testInfo) => {
    test.setTimeout(180_000);
    await using user = await createUserWithWorkspace(api);
    const token = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));
    const repo = await createContainerRepo(api, user, user.workspaceId);

    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: token,
    });

    const tags = await listTagsAPI(api, user, user.workspaceId, repo.id);
    expect(tags.map((t) => t.tag)).toContain('v1');

    const manifests = await listManifestsAPI(api, user, user.workspaceId, repo.id);
    expect(manifests.length).toBe(1);
    expect(manifests[0].tags).toContain('v1');
    expect(manifests[0].digest).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(manifests[0].size).toBeGreaterThan(0);

    // Pull it back through the registry.
    const pulled = await pullImageFromPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: token,
    });
    expect(pulled.ok).toBe(true);

    // traefik/whoami declares EXPOSE 80.
    const ports = await exposedPortsAPI(api, user, user.workspaceId, repo.id, 'v1');
    expect(ports).toContain(80);
  });

  test('two tags on one image share a digest; re-push moves the tag', async ({ api }, testInfo) => {
    test.setTimeout(180_000);
    await using user = await createUserWithWorkspace(api);
    const token = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));
    const repo = await createContainerRepo(api, user, user.workspaceId);

    // Same source pushed under two tags → two tag rows, one manifest digest.
    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: token,
    });
    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v2',
      apiToken: token,
    });

    let tags = await listTagsAPI(api, user, user.workspaceId, repo.id);
    expect(tags.map((t) => t.tag).sort()).toEqual(['v1', 'v2']);
    const digestV1 = tags.find((t) => t.tag === 'v1')!.digest;
    expect(tags.find((t) => t.tag === 'v2')!.digest).toBe(digestV1);
    let manifests = await listManifestsAPI(api, user, user.workspaceId, repo.id);
    expect(manifests.length).toBe(1);

    // Push a different image under v1 → the v1 tag moves to the new digest.
    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: token,
      sourceImage: SECOND_IMAGE,
    });
    tags = await listTagsAPI(api, user, user.workspaceId, repo.id);
    expect(tags.find((t) => t.tag === 'v1')!.digest).not.toBe(digestV1);
    expect(tags.find((t) => t.tag === 'v2')!.digest).toBe(digestV1);
    manifests = await listManifestsAPI(api, user, user.workspaceId, repo.id);
    expect(manifests.length).toBe(2);
  });

  test('login rejects a wrong username and a bad token', async ({ api }, testInfo) => {
    test.setTimeout(120_000);
    await using user = await createUserWithWorkspace(api);
    const token = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));

    // Realm requires username == "patr".
    const wrongUser = await dockerLoginPatr(dind.dockerHost, token, 'notpatr');
    expect(wrongUser.ok).toBe(false);

    // Garbage token.
    const badToken = await dockerLoginPatr(dind.dockerHost, 'patrv1.not.a.real.token');
    expect(badToken.ok).toBe(false);

    // The real token + correct username works.
    const good = await dockerLoginPatr(dind.dockerHost, token);
    expect(good.ok).toBe(true);
  });

  test('push authorization: nonexistent repo, pull-only token, cross-workspace all fail', async ({
    api,
  }, testInfo) => {
    test.setTimeout(180_000);
    await using user = await createUserWithWorkspace(api);
    const adminToken = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));

    // Push to a repo that was never created → no auto-create.
    const ghost = await tryPushImage({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: randomRepoName('ghost'),
      tag: 'v1',
      apiToken: adminToken,
    });
    expect(ghost.ok).toBe(false);

    // Pull-only token cannot push (existence hidden → push fails).
    const repo = await createContainerRepo(api, user, user.workspaceId);
    const pullToken = await scopedToken(api, user, 'containerRegistryRepository::pull');
    const pushWithPull = await tryPushImage({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: pullToken,
    });
    expect(pushWithPull.ok).toBe(false);

    // A second workspace's repo id is not pushable with the first user's token
    // under the first workspace path; pushing to the other workspace path fails.
    await using other = await createUserWithWorkspace(api);
    const otherRepo = await createContainerRepo(api, other, other.workspaceId);
    const crossWs = await tryPushImage({
      dockerHost: dind.dockerHost,
      workspaceId: other.workspaceId,
      repoName: otherRepo.name,
      tag: 'v1',
      apiToken: adminToken, // token scoped to `user`'s workspace, not `other`
    });
    expect(crossWs.ok).toBe(false);
  });

  test('pull authorization: a push-only token cannot pull', async ({ api }, testInfo) => {
    test.setTimeout(180_000);
    await using user = await createUserWithWorkspace(api);
    const adminToken = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));
    const repo = await createContainerRepo(api, user, user.workspaceId);

    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: adminToken,
    });

    const pushToken = await scopedToken(api, user, 'containerRegistryRepository::push');
    const pulled = await pullImageFromPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'v1',
      apiToken: pushToken,
    });
    expect(pulled.ok).toBe(false);
  });
});

test.describe('@docker container registry images tab [UI]', () => {
  test('a pushed image appears in the Images tab with its tag and digest', async ({
    api,
    browser,
  }, testInfo) => {
    test.setTimeout(180_000);
    await using user = await createUserWithWorkspace(api);
    const token = await superAdminToken(api, user);
    await using dind = await DindHandle.spawn(dv(testInfo));
    const repo = await createContainerRepo(api, user, user.workspaceId);
    await pushImageToPatrRegistry({
      dockerHost: dind.dockerHost,
      workspaceId: user.workspaceId,
      repoName: repo.name,
      tag: 'release',
      apiToken: token,
    });

    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRegistryDetail(page, repo.id, 'images');
      await expect(imagesTab(page)).toBeVisible();
      await expect(page.getByText('release', { exact: false }).first()).toBeVisible();
      // The digest is rendered (sha256-prefixed) in the manifest row.
      await expect(page.getByText(/sha256:/).first()).toBeVisible();
    } finally {
      await context.close();
    }
  });
});
