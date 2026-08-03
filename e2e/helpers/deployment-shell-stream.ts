import WebSocket, { type RawData } from 'ws';
import { API_DIRECT_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

const WS_BASE = API_DIRECT_URL.replace(/^http/, 'ws');

/** A single frame off the deployment-shell socket (`#[serde(tag = "type")]`). */
export type ShellMsg = { type: string; [k: string]: unknown };

/**
 * Websocket client for the CLI-facing deployment-shell endpoint
 * (`GET /workspace/{workspaceId}/deployment/{deploymentId}/shell`). Mirrors
 * {@link RunnerStream}: JSON text frames, one per typed message. Terminal bytes
 * (`Stdin`/`Output`) are base64 — this helper encodes/decodes them for you and
 * accumulates decoded output so tests can wait on substrings.
 */
export class DeploymentShellStream implements AsyncDisposable {
  private readonly ws: WebSocket;
  private readonly inbox: ShellMsg[] = [];
  private waiter: { pred: (m: ShellMsg) => boolean; resolve: (m: ShellMsg) => void } | null = null;
  /** All decoded `Output` bytes received so far, as a UTF-8 string. */
  public output = '';
  /** Set once the server closes the socket (e.g. after the session ends). */
  public closed = false;

  private constructor(ws: WebSocket) {
    this.ws = ws;
    ws.on('close', () => {
      this.closed = true;
    });
    ws.on('message', (data: RawData) => {
      const msg = JSON.parse(data.toString()) as ShellMsg;
      if (msg.type === 'Output' && typeof msg.data === 'string') {
        this.output += Buffer.from(msg.data, 'base64').toString('utf8');
      }
      if (this.waiter && this.waiter.pred(msg)) {
        const { resolve } = this.waiter;
        this.waiter = null;
        resolve(msg);
      } else {
        this.inbox.push(msg);
      }
    });
  }

  static open(opts: {
    workspaceId: string;
    deploymentId: string;
    token: string;
    clientIp: string;
  }): Promise<DeploymentShellStream> {
    const url = `${WS_BASE}/workspace/${opts.workspaceId}/deployment/${opts.deploymentId}/shell`;
    const ws = new WebSocket(url, {
      headers: {
        Authorization: `Bearer ${opts.token}`,
        'User-Agent': USER_AGENT,
        'X-Real-IP': opts.clientIp,
      },
    });
    return new Promise((resolve, reject) => {
      ws.on('open', () => resolve(new DeploymentShellStream(ws)));
      ws.on('unexpected-response', (_req, res) => reject(new Error(`WS ${res.statusCode}`)));
      ws.on('error', reject);
    });
  }

  /** Send a `Stdin` frame (text is base64-encoded on the wire). */
  sendStdin(text: string): void {
    this.ws.send(
      JSON.stringify({ type: 'Stdin', data: Buffer.from(text, 'utf8').toString('base64') }),
    );
  }

  /** Send a terminal `Resize`. */
  sendResize(rows: number, cols: number): void {
    this.ws.send(JSON.stringify({ type: 'Resize', rows, cols }));
  }

  /** Resolve with the next frame matching `pred` (default: any). */
  next(pred: (m: ShellMsg) => boolean = () => true, timeoutMs = 10_000): Promise<ShellMsg> {
    const buffered = this.inbox.findIndex(pred);
    if (buffered >= 0) return Promise.resolve(this.inbox.splice(buffered, 1)[0]);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiter = null;
        reject(new Error('timed out waiting for a shell frame'));
      }, timeoutMs);
      this.waiter = {
        pred,
        resolve: (m) => {
          clearTimeout(timer);
          resolve(m);
        },
      };
    });
  }

  /** Wait until the accumulated decoded output contains `needle`. */
  async waitForOutput(needle: string, timeoutMs = 15_000): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    while (!this.output.includes(needle)) {
      if (Date.now() >= deadline) {
        throw new Error(`output never contained ${JSON.stringify(needle)}; got:\n${this.output}`);
      }
      await this.next(() => true, Math.max(1, deadline - Date.now())).catch(() => {});
    }
  }

  async close(): Promise<void> {
    this.ws.close();
  }

  async [Symbol.asyncDispose](): Promise<void> {
    this.ws.terminate();
  }
}
