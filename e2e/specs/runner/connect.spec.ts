import {
  test,
  expect,
  newContext,
  createUserWithWorkspace,
  loginAs,
  RunnerHandle,
} from '@/prelude';
import type { DockerVersion } from '@/prelude';
import { isRunnerConnected } from '@/helpers/runner';
import { listRunnersAPI } from '@/helpers/runner-api';
import { openRunnerDetail, statusBadge } from '@/helpers/ui/runner';

function dv(testInfo: { project: { metadata: Record<string, unknown> } }): DockerVersion {
  return (testInfo.project.metadata.dockerVersion ?? '26') as DockerVersion;
}

// A runner's websocket health is verified by observing that it connects and
// stays stable (the dashboard shows it Online and it keeps reporting connected)
// — not by speaking the raw stream protocol.
test.describe('@docker runner connection lifecycle', () => {
  test('a real runner connects and stays stable (Online in the dashboard)', async ({
    api,
    browser,
  }, testInfo) => {
    test.setTimeout(120_000);
    await using user = await createUserWithWorkspace(api);
    await using runner = await RunnerHandle.connect({
      api,
      user,
      workspaceId: user.workspaceId,
      dockerVersion: dv(testInfo),
    });

    // The detail UI shows the Online badge.
    const context = await newContext(browser, user.clientIp);
    await loginAs(context, user, { workspaceId: user.workspaceId });
    const page = await context.newPage();
    try {
      await openRunnerDetail(page, runner.runnerId, undefined);
      await expect(statusBadge(page, 'Online')).toBeVisible({ timeout: 15_000 });

      // Stable: still connected after a settle period well past the debug lock
      // TTL (5s) — i.e. the renewal loop is holding the connection, not a stale
      // lock about to expire.
      await new Promise((r) => setTimeout(r, 8000));
      expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(true);

      const list = await listRunnersAPI(api, user, user.workspaceId, '?page=0&count=100');
      const row = list.runners.find((r) => r.id === runner.runnerId);
      expect(row?.connected).toBe(true);
      expect(row?.lastSeen).not.toBeNull();
    } finally {
      await context.close();
    }
  });

  test('graceful shutdown releases the lock → runner flips to disconnected', async ({
    api,
  }, testInfo) => {
    test.setTimeout(120_000);
    await using user = await createUserWithWorkspace(api);
    const runner = await RunnerHandle.connect({
      api,
      user,
      workspaceId: user.workspaceId,
      dockerVersion: dv(testInfo),
    });
    let disposed = false;
    try {
      expect(await isRunnerConnected(api, user, user.workspaceId, runner.runnerId)).toBe(true);

      // Graceful SIGTERM: the runner releases its connection lock on shutdown.
      await runner[Symbol.asyncDispose]();
      disposed = true;

      // connected (lock-derived) flips to false promptly.
      let connected = true;
      for (let i = 0; i < 15; i++) {
        connected = await isRunnerConnected(api, user, user.workspaceId, runner.runnerId);
        if (!connected) break;
        await new Promise((r) => setTimeout(r, 1000));
      }
      expect(connected).toBe(false);
    } finally {
      if (!disposed) await runner[Symbol.asyncDispose]();
    }
  });
});
