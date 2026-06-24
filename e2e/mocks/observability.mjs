// Standalone Loki + Mimir stub for the e2e stack. The API reads deployment /
// runner logs from Loki (`/loki/api/v1/query_range`, `/loki/api/v1/tail`) and
// metrics from Mimir (`/prometheus/api/v1/query_range`). Pointing
// PATR__OPENTELEMETRY__LOGS__ENDPOINT and __METRICS__ENDPOINT at this server
// lets logs/metrics specs:
//   - return deterministic data (per-workspace canned values),
//   - exercise the parse-error path (malformed mode),
//   - assert the exact LogQL/PromQL + step/limit and the x-scope-orgid header
//     (every request is recorded),
//   - and still see real data end-to-end: an un-configured workspace is
//     proxied to the real Loki/Mimir compose services (REAL_LOKI / REAL_MIMIR).
//
// State is keyed by the x-scope-orgid header (= workspace id) so parallel tests
// don't collide. Control plane: POST /__configure, GET /__requests, POST /__reset.
// Plain node:http (+ hand-rolled WS frames for the tail), no deps.
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';

const PORT = Number(process.env.OBS_MOCK_PORT ?? 13900);
const REAL_LOKI = process.env.REAL_LOKI ?? 'http://127.0.0.1:13100';
const REAL_MIMIR = process.env.REAL_MIMIR ?? 'http://127.0.0.1:18080';

/** org -> { loki?, mimir?, tail?, malformed? } */
const configs = new Map();
/** org -> [{ kind, path, query, headers }] */
const requests = new Map();

function cfg(org) {
  return configs.get(org) ?? {};
}
function record(org, entry) {
  if (!requests.has(org)) requests.set(org, []);
  requests.get(org).push(entry);
}
// query_range response the handlers expect: { data: { result: [{ values: [...] }] } }
function rangeBody(values) {
  return JSON.stringify({
    status: 'success',
    data: { resultType: 'matrix', result: values && values.length ? [{ metric: {}, values }] : [] },
  });
}

async function readJson(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  const raw = Buffer.concat(chunks).toString('utf8');
  return raw ? JSON.parse(raw) : {};
}

// Forward a request (method/path/query/body/x-scope-orgid) to a real backend.
// Used both for query_range proxy-fallback and the catch-all (so the API's own
// OTLP-HTTP log export and the loki/mimir proxy upstreams keep working when this
// stub stands in front of the real endpoints).
async function proxy(realBase, req, res, search, body) {
  try {
    const headers = {};
    if (req.headers['x-scope-orgid'])
      headers['x-scope-orgid'] = String(req.headers['x-scope-orgid']);
    if (req.headers['content-type']) headers['content-type'] = String(req.headers['content-type']);
    const upstream = await fetch(`${realBase}${req.url.split('?')[0]}${search}`, {
      method: req.method,
      headers,
      body: req.method === 'GET' || req.method === 'HEAD' ? undefined : body,
    });
    const buf = Buffer.from(await upstream.arrayBuffer());
    res.writeHead(upstream.status, {
      'Content-Type': upstream.headers.get('content-type') ?? 'application/json',
    });
    res.end(buf);
  } catch (err) {
    res.writeHead(502, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'error', error: `obs-mock proxy failed: ${err}` }));
  }
}

async function readBody(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  return Buffer.concat(chunks);
}

const server = createServer(async (req, res) => {
  const [path, search = ''] = (req.url ?? '').split(/\?(.*)/s);
  const org = String(req.headers['x-scope-orgid'] ?? '');

  // --- control plane ---
  if (path === '/__configure' && req.method === 'POST') {
    const body = await readJson(req);
    const { org: o, ...rest } = body;
    configs.set(String(o ?? ''), rest);
    res.writeHead(200).end('{"ok":true}');
    return;
  }
  if (path === '/__requests' && req.method === 'GET') {
    const o = new URL(req.url, 'http://x').searchParams.get('org') ?? '';
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(requests.get(o) ?? []));
    return;
  }
  if (path === '/__reset' && req.method === 'POST') {
    const o = new URL(req.url, 'http://x').searchParams.get('org') ?? '';
    configs.delete(o);
    requests.delete(o);
    res.writeHead(200).end('{"ok":true}');
    return;
  }

  const params = Object.fromEntries(new URLSearchParams(search));

  // --- Loki query_range ---
  if (path === '/loki/api/v1/query_range') {
    record(org, { kind: 'loki', path, query: params, headers: { 'x-scope-orgid': org } });
    const c = cfg(org);
    if (c.malformed === 'loki') return void res.writeHead(200).end('not json at all');
    if (c.loki)
      return void res
        .writeHead(200, { 'Content-Type': 'application/json' })
        .end(rangeBody(c.loki.values));
    return void proxy(REAL_LOKI, req, res, search ? `?${search}` : '');
  }

  // --- Mimir query_range ---
  if (path === '/prometheus/api/v1/query_range') {
    record(org, { kind: 'mimir', path, query: params, headers: { 'x-scope-orgid': org } });
    const c = cfg(org);
    if (c.malformed === 'mimir') return void res.writeHead(200).end('not json at all');
    if (c.mimir)
      return void res
        .writeHead(200, { 'Content-Type': 'application/json' })
        .end(rangeBody(c.mimir.values));
    return void proxy(REAL_MIMIR, req, res, search ? `?${search}` : '');
  }

  // Catch-all: proxy anything else (OTLP log export, loki/mimir proxy
  // upstreams, etc.) to the real backend so standing in front of the real
  // endpoints is transparent. Route by path; default to Loki.
  const realBase = path.startsWith('/prometheus') ? REAL_MIMIR : REAL_LOKI;
  const body = req.method === 'GET' || req.method === 'HEAD' ? undefined : await readBody(req);
  return void proxy(realBase, req, res, search ? `?${search}` : '', body);
});

// --- Loki tail (server→client websocket) ---
// Minimal hand-rolled WS: complete the handshake, then push the org's configured
// tail frames as Loki tail messages ({streams:[{values:[[ns,line]]}]}). Server
// frames are unmasked text frames; we never read client frames.
server.on('upgrade', (req, socket) => {
  const [path, search = ''] = (req.url ?? '').split(/\?(.*)/s);
  const org = String(req.headers['x-scope-orgid'] ?? '');
  if (path !== '/loki/api/v1/tail') {
    socket.destroy();
    return;
  }
  record(org, {
    kind: 'loki-tail',
    path,
    query: Object.fromEntries(new URLSearchParams(search)),
    headers: { 'x-scope-orgid': org },
  });

  const key = req.headers['sec-websocket-key'];
  const accept = createHash('sha1')
    .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
    .digest('base64');
  socket.write(
    'HTTP/1.1 101 Switching Protocols\r\n' +
      'Upgrade: websocket\r\nConnection: Upgrade\r\n' +
      `Sec-WebSocket-Accept: ${accept}\r\n\r\n`,
  );

  const tail = cfg(org).tail ?? [];
  for (const values of tail) {
    sendTextFrame(socket, JSON.stringify({ streams: [{ stream: {}, values }] }));
  }
  // Leave the socket open; the API closes it when the client disconnects.
  socket.on('error', () => socket.destroy());
});

function sendTextFrame(socket, text) {
  const payload = Buffer.from(text, 'utf8');
  const len = payload.length;
  let header;
  if (len < 126) {
    header = Buffer.from([0x81, len]);
  } else if (len < 65536) {
    header = Buffer.from([0x81, 126, (len >> 8) & 0xff, len & 0xff]);
  } else {
    header = Buffer.alloc(10);
    header[0] = 0x81;
    header[1] = 127;
    header.writeBigUInt64BE(BigInt(len), 2);
  }
  socket.write(Buffer.concat([header, payload]));
}

server.listen(PORT, () => {
  console.log(`observability mock (loki+mimir) listening on http://127.0.0.1:${PORT}`);
});
