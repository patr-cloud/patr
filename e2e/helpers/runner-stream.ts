import WebSocket, { type RawData } from 'ws';
import { API_DIRECT_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

// Raw authenticated client for the runner data stream
// (GET /workspace/{ws}/runner/{id}/stream). The real runner uses an API token
// over this websocket; these helpers let specs drive the handshake/lock
// behavior (exposure handshake, RunnerAlreadyConnected, lock lifecycle) without
// spinning a DinD runner.
//
// Wire format (axum_typed_websockets JSON codec): each typed message is a JSON
// text frame. The server/client message enums are `#[serde(tag = "type")]` with
// the *exact* variant name as the tag (no rename), and variant fields stay
// snake_case. So SetRunnerExposureType is:
//   {"type":"SetRunnerExposureType","exposure_type":{"type":"private"}}
// and the server's prompt is {"type":"ExposureTypeRequired"}.
//
// Auth + lock both happen before the upgrade: a missing Execute permission is a
// 401 and a second concurrent connection is a 409 (RunnerAlreadyConnected),
// surfaced as an HTTP rejection (no 101) — open() rejects with the status.

const WS_BASE = API_DIRECT_URL.replace(/^http/, 'ws');

export type StreamMsg = { type: string; [k: string]: unknown };

export type ExposureType =
	| { type: 'private' }
	| { type: 'publicIP'; ipAddresses: string[] }
	| { type: 'publicDNS'; dnsName: string };

export class RunnerStream implements AsyncDisposable {
	private readonly inbox: StreamMsg[] = [];
	private readonly waiters: Array<{
		pred: (m: StreamMsg) => boolean;
		resolve: (m: StreamMsg) => void;
	}> = [];
	private closeInfo: { code: number } | null = null;

	private constructor(private readonly ws: WebSocket) {
		ws.on('message', (data: RawData) => {
			let msg: StreamMsg;
			try {
				msg = JSON.parse(data.toString()) as StreamMsg;
			} catch {
				return;
			}
			const idx = this.waiters.findIndex((w) => w.pred(msg));
			if (idx >= 0) {
				const [w] = this.waiters.splice(idx, 1);
				w.resolve(msg);
			} else {
				this.inbox.push(msg);
			}
		});
		ws.on('close', (code: number) => {
			this.closeInfo = { code };
		});
	}

	// Opens the stream. Resolves once upgraded, rejects with `WS <status>` if the
	// server refuses the upgrade (401 no-Execute, 409 RunnerAlreadyConnected).
	static open(opts: {
		workspaceId: string;
		runnerId: string;
		token: string;
		clientIp: string;
	}): Promise<RunnerStream> {
		const url = `${WS_BASE}/workspace/${opts.workspaceId}/runner/${opts.runnerId}/stream`;
		const ws = new WebSocket(url, {
			headers: {
				Authorization: `Bearer ${opts.token}`,
				'User-Agent': USER_AGENT,
				'X-Real-IP': opts.clientIp,
			},
		});
		return new Promise((resolve, reject) => {
			let settled = false;
			ws.on('open', () => {
				if (settled) return;
				settled = true;
				resolve(new RunnerStream(ws));
			});
			ws.on('unexpected-response', (_req, res) => {
				if (settled) return;
				settled = true;
				reject(new Error(`WS ${res.statusCode}`));
				ws.terminate();
			});
			ws.on('error', (err) => {
				if (settled) return;
				settled = true;
				reject(err);
			});
		});
	}

	send(obj: unknown): void {
		this.ws.send(JSON.stringify(obj));
	}

	sendExposureType(exposure: ExposureType = { type: 'private' }): void {
		// The wire enum uses snake_case field names; the nested exposure type is
		// camelCase-tagged (private/publicIP/publicDNS).
		const exposure_type =
			exposure.type === 'private'
				? { type: 'private' }
				: exposure.type === 'publicIP'
					? { type: 'publicIP', ip_addresses: exposure.ipAddresses }
					: { type: 'publicDNS', dns_name: exposure.dnsName };
		this.send({ type: 'SetRunnerExposureType', exposure_type });
	}

	// Waits for the next inbound message matching `pred` (default: any).
	next(pred: (m: StreamMsg) => boolean = () => true, timeoutMs = 5000): Promise<StreamMsg> {
		const idx = this.inbox.findIndex(pred);
		if (idx >= 0) return Promise.resolve(this.inbox.splice(idx, 1)[0]);
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				const i = this.waiters.findIndex((w) => w.resolve === wrapped);
				if (i >= 0) this.waiters.splice(i, 1);
				reject(new Error(`timed out waiting for stream message after ${timeoutMs}ms`));
			}, timeoutMs);
			const wrapped = (m: StreamMsg) => {
				clearTimeout(timer);
				resolve(m);
			};
			this.waiters.push({ pred, resolve: wrapped });
		});
	}

	// Closes gracefully (server runs its compare-and-delete lock release) and
	// waits for the close to land.
	async close(): Promise<void> {
		if (this.closeInfo) return;
		this.ws.close();
		for (let i = 0; i < 50 && !this.closeInfo; i++) {
			await new Promise((r) => setTimeout(r, 100));
		}
	}

	async [Symbol.asyncDispose](): Promise<void> {
		try {
			this.ws.terminate();
		} catch {
			// already closed
		}
	}
}
