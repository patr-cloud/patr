import { ChildProcess } from 'node:child_process';
import { createConnection } from 'node:net';

export async function waitForPort(
  port: number,
  opts: { host?: string; timeoutMs?: number; label?: string } = {},
): Promise<void> {
  const host = opts.host ?? '127.0.0.1';
  const deadline = Date.now() + (opts.timeoutMs ?? 15_000);

  while (Date.now() < deadline) {
    const ok = await new Promise<boolean>((resolve) => {
      const socket = createConnection({ port, host });
      socket.once('connect', () => { socket.end(); resolve(true); });
      socket.once('error', () => resolve(false));
    });
    if (ok) return;
    await new Promise((r) => setTimeout(r, 100));
  }

  throw new Error(`timed out waiting for ${opts.label ?? `${host}:${port}`}`);
}

export async function waitFor(
  predicate: () => Promise<boolean>,
  opts: { timeoutMs?: number; intervalMs?: number; label?: string } = {},
): Promise<void> {
  const deadline = Date.now() + (opts.timeoutMs ?? 15_000);
  const interval = opts.intervalMs ?? 200;

  while (Date.now() < deadline) {
    try {
      if (await predicate()) return;
    } catch {
      // keep polling
    }
    await new Promise((r) => setTimeout(r, interval));
  }

  throw new Error(`timed out waiting for ${opts.label ?? 'condition'}`);
}

export function onExit(proc: ChildProcess): Promise<number | null> {
  return new Promise((resolve) => {
    if (proc.exitCode !== null) return resolve(proc.exitCode);
    proc.once('exit', (code) => resolve(code));
  });
}
