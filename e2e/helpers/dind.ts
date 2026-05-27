import { execa } from 'execa';
import { waitFor } from '@/helpers/process';

export type DockerVersion = '24' | '25' | '26';

export class DindHandle implements AsyncDisposable {
  private constructor(
    public readonly containerId: string,
    public readonly hostPort: number,
    public readonly version: DockerVersion,
  ) {}

  static async spawn(version: DockerVersion): Promise<DindHandle> {
    const name = `e2e-dind-${crypto.randomUUID().slice(0, 8)}`;

    const { stdout: cid } = await execa('docker', [
      'run', '--rm', '-d', '--privileged',
      '--name', name,
      '-p', '0:2375',
      `docker:${version}-dind`,
      'dockerd', '--host=tcp://0.0.0.0:2375',
    ]);

    const containerId = cid.trim();

    let hostPort: number;
    try {
      const { stdout: portMap } = await execa('docker', [
        'port', containerId, '2375/tcp',
      ]);
      const match = portMap.match(/:(\d+)$/m);
      if (!match) throw new Error(`could not parse port mapping: ${portMap}`);
      hostPort = Number(match[1]);

      await waitFor(
        async () => {
          const res = await fetch(`http://127.0.0.1:${hostPort}/_ping`);
          return res.ok;
        },
        { timeoutMs: 30_000, label: `dind ${version} ready` },
      );
    } catch (err) {
      await execa('docker', ['rm', '-f', containerId]).catch(() => {});
      throw err;
    }

    return new DindHandle(containerId, hostPort, version);
  }

  get dockerHost(): string {
    return `tcp://127.0.0.1:${this.hostPort}`;
  }

  async [Symbol.asyncDispose](): Promise<void> {
    await execa('docker', ['rm', '-f', this.containerId]).catch(() => {});
  }
}
