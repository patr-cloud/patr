import { ChildProcess, spawn } from 'node:child_process';
import { openSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import type { ApiClient } from '@/helpers/api';
import { DindHandle, type DockerVersion } from '@/helpers/dind';
import { onExit, waitFor } from '@/helpers/process';
import type { UserHandle } from '@/helpers/user';

const REPO_ROOT = resolve(import.meta.dirname, '..', '..');
const RUNNER_BINARY = join(REPO_ROOT, 'target', 'debug', 'docker');

// Pool of bind ports per worker. The runner's own HTTP listener; each parallel
// runner needs its own. We pick from an ephemeral-ish range above 24000.
let nextBindPort = 24000;
function allocateBindPort(): number {
  return nextBindPort++;
}

export type RunnerOpts = {
  api: ApiClient;
  user: UserHandle;
  dockerVersion: DockerVersion;
};

export class RunnerHandle implements AsyncDisposable {
  private constructor(
    private readonly proc: ChildProcess,
    private readonly dind: DindHandle,
    public readonly runnerId: string,
    public readonly workspaceId: string,
    public readonly bindPort: number,
    private readonly logDir: string,
  ) {}

  static async connect(opts: RunnerOpts): Promise<RunnerHandle> {
    const dind = await DindHandle.spawn(opts.dockerVersion);

    let workspaceId: string;
    let runnerId: string;
    let apiToken: string;
    try {
      ({ workspaceId, runnerId, apiToken } = await provisionRunner(opts));

      const bindPort = allocateBindPort();
      const logDir = mkdtempSync(join(tmpdir(), 'patr-e2e-runner-'));
      const dbPath = join(logDir, 'runner.db');

      const proc = spawn(RUNNER_BINARY, [], {
        cwd: REPO_ROOT,
        env: {
          ...process.env,
          DOCKER_HOST: dind.dockerHost,
          PATR_BIND_ADDRESS: `127.0.0.1:${bindPort}`,
          PATR_MODE: 'managed',
          PATR_WORKSPACE_ID: workspaceId,
          PATR_RUNNER_ID: runnerId,
          PATR_API_TOKEN: apiToken,
          PATR_DATABASE_FILE: dbPath,
          PATR_DATABASE_CONNECTION_LIMIT: '5',
        },
        stdio: [
          'ignore',
          openSync(join(logDir, 'stdout.log'), 'w'),
          openSync(join(logDir, 'stderr.log'), 'w'),
        ],
      });

      proc.once('exit', (code) => {
        if (code !== null && code !== 0) {
          console.error(
            `runner ${runnerId} exited early with code ${code}; ` + `logs at ${logDir}`,
          );
        }
      });

      await waitFor(async () => isRunnerConnected(opts.api, opts.user, runnerId), {
        timeoutMs: 30_000,
        label: `runner ${runnerId} connected`,
      });

      return new RunnerHandle(proc, dind, runnerId, workspaceId, bindPort, logDir);
    } catch (err) {
      await dind[Symbol.asyncDispose]();
      throw err;
    }
  }

  async [Symbol.asyncDispose](): Promise<void> {
    this.proc.kill('SIGTERM');
    await onExit(this.proc);
    await this.dind[Symbol.asyncDispose]();
  }
}

async function provisionRunner(opts: RunnerOpts): Promise<{
  workspaceId: string;
  runnerId: string;
  apiToken: string;
}> {
  // NOTE: workspace + runner creation endpoints — exact request shapes need
  // to be wired up once we have a deploy spec. For now this throws if called
  // (login spec doesn't use this path).
  throw new Error('provisionRunner not yet implemented — wire up when adding first @docker spec');
}

async function isRunnerConnected(
  api: ApiClient,
  user: UserHandle,
  runnerId: string,
): Promise<boolean> {
  // Placeholder; wire up alongside provisionRunner.
  return false;
}
