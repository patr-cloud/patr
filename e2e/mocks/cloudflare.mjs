// Standalone Cloudflare API mock for the e2e stack. The real API would call
// out to api.cloudflare.com for tunnels (runner ingress), custom hostnames
// (managed URLs), zones (internal domains), Workers KV, and DNS records. In
// e2e we point PATR__CLOUDFLARE__BASE_URL at this server so those calls
// succeed deterministically.
//
// This mirrors `mount_cloudflare_mocks` in api/tests/setup.rs — keep the two in
// sync. Plain node:http, no deps, runnable with `node mocks/cloudflare.mjs`.
//
// Special behaviours preserved from the Rust mock:
// - A custom hostname id of `pending-hostname-id` reports status "pending"
//   (lets managed-URL verify-configuration exercise the not-configured path);
//   every other id reports "active" on PATCH.
import { createServer } from 'node:http';

const PORT = Number(process.env.CF_MOCK_PORT ?? 18888);

function envelope(result, resultInfo) {
  const body = { success: true, errors: [], messages: [], result };
  if (resultInfo) body.result_info = resultInfo;
  return JSON.stringify(body);
}

// A complete Zone per the cloudflare crate's Zone struct — many fields are
// required (account, owner, meta, the timestamps, permissions, status), so a
// minimal object fails to decode and breaks ListZones/CreateZone.
const ZONE = {
  id: 'mock-zone-id',
  name: 'testonpatr.cloud',
  account: { id: 'mock-account-id', name: 'Mock Account' },
  activated_on: '2024-01-01T00:00:00Z',
  created_on: '2024-01-01T00:00:00Z',
  development_mode: 0,
  meta: { custom_certificate_quota: 0, page_rule_quota: 0, phishing_detected: false },
  modified_on: '2024-01-01T00:00:00Z',
  name_servers: ['ns1.mock.com', 'ns2.mock.com'],
  owner: { type: 'user', id: 'mock-owner-id', email: 'mock@example.com' },
  paused: false,
  permissions: [],
  status: 'active',
  type: 'full',
};
const TUNNEL = {
  id: '00000000-0000-0000-0000-000000000000',
  name: 'mock-tunnel',
  created_at: '2024-01-01T00:00:00Z',
  deleted_at: null,
  connections: [],
  metadata: {},
};
const LIST_INFO = { page: 1, per_page: 20, total_pages: 1, count: 1, total_count: 1 };
// A complete DnsRecord per the cloudflare crate (meta{}, flattened type+content,
// timestamps, etc.) — CreateDnsRecord/UpdateDnsRecord decode the result as a
// single DnsRecord object, so returning a bare list breaks decoding.
const DNS_RECORD = {
  meta: {},
  name: 'mock.onpatr.cloud',
  ttl: 1,
  modified_on: '2024-01-01T00:00:00Z',
  created_on: '2024-01-01T00:00:00Z',
  proxiable: true,
  type: 'CNAME',
  content: 'mock.cfargotunnel.com',
  id: 'mock-dns-record-id',
  proxied: true,
};

// Ordered [method-or-null, regex, handler] — first match wins, so specific
// patterns (e.g. the pending hostname, the tunnel token) precede general ones.
const routes = [
  // custom hostnames (managed URLs)
  [
    'POST',
    /^\/client\/v4\/zones\/[^/]+\/custom_hostnames$/,
    () =>
      envelope({
        id: 'mock-custom-hostname-id',
        hostname: 'example.com',
        ssl: { status: 'pending_validation', method: 'txt', type: 'dv', validation_records: [] },
        status: 'pending',
      }),
  ],
  [
    'PATCH',
    /^\/client\/v4\/zones\/[^/]+\/custom_hostnames\/pending-hostname-id$/,
    () =>
      envelope({
        id: 'pending-hostname-id',
        hostname: 'example.com',
        ssl: { status: 'pending_validation', method: 'txt', type: 'dv', validation_records: [] },
        status: 'pending',
      }),
  ],
  [
    'PATCH',
    /^\/client\/v4\/zones\/[^/]+\/custom_hostnames\/[^/]+$/,
    () =>
      envelope({
        id: 'mock-custom-hostname-id',
        hostname: 'example.com',
        ssl: { status: 'active', method: 'txt', type: 'dv', validation_records: [] },
        status: 'active',
      }),
  ],
  [
    'GET',
    /^\/client\/v4\/zones\/[^/]+\/custom_hostnames\/[^/]+$/,
    () =>
      envelope({
        id: 'mock-custom-hostname-id',
        hostname: 'example.com',
        ssl: {
          status: 'pending_validation',
          method: 'txt',
          type: 'dv',
          validation_records: [
            { txt_name: '_acme-challenge.example.com', txt_value: 'mock-txt-value' },
          ],
        },
        status: 'pending',
        ownership_verification: {
          type: 'txt',
          name: '_cf-custom-hostname.example.com',
          value: 'mock-ownership-value',
        },
      }),
  ],
  [
    'DELETE',
    /^\/client\/v4\/zones\/[^/]+\/custom_hostnames\/[^/]+$/,
    () => envelope({ id: 'mock-custom-hostname-id' }),
  ],
  // dns records — method-specific (before the bare-zone patterns). Create and
  // update return a single DnsRecord; list returns an array; delete returns {id}.
  ['POST', /^\/client\/v4\/zones\/[^/]+\/dns_records$/, () => envelope(DNS_RECORD)],
  [
    'GET',
    /^\/client\/v4\/zones\/[^/]+\/dns_records$/,
    () => envelope([], { ...LIST_INFO, count: 0, total_count: 0 }),
  ],
  ['PUT', /^\/client\/v4\/zones\/[^/]+\/dns_records\/[^/]+$/, () => envelope(DNS_RECORD)],
  ['PATCH', /^\/client\/v4\/zones\/[^/]+\/dns_records\/[^/]+$/, () => envelope(DNS_RECORD)],
  [
    'DELETE',
    /^\/client\/v4\/zones\/[^/]+\/dns_records\/[^/]+$/,
    () => envelope({ id: DNS_RECORD.id }),
  ],
  // zones (internal domains)
  ['GET', /^\/client\/v4\/zones$/, () => envelope([ZONE], LIST_INFO)],
  ['POST', /^\/client\/v4\/zones$/, () => envelope(ZONE)],
  ['DELETE', /^\/client\/v4\/zones\/[^/]+$/, () => envelope({ id: 'mock-zone-id' })],
  // Workers KV (deployment/managed-url ingress config)
  [
    'PUT',
    /^\/client\/v4\/accounts\/[^/]+\/storage\/kv\/namespaces\/[^/]+\/values\/.+$/,
    () => envelope(null),
  ],
  [
    'DELETE',
    /^\/client\/v4\/accounts\/[^/]+\/storage\/kv\/namespaces\/[^/]+\/values\/.+$/,
    () => envelope(null),
  ],
  // cloudflare tunnels (runner ingress) — token route before the general one
  [
    'GET',
    /^\/client\/v4\/accounts\/[^/]+\/cfd_tunnel\/[^/]+\/token$/,
    () => envelope('mock-tunnel-token-value'),
  ],
  ['POST', /^\/client\/v4\/accounts\/[^/]+\/cfd_tunnel$/, () => envelope(TUNNEL)],
  ['PUT', /^\/client\/v4\/accounts\/[^/]+\/cfd_tunnel\/[^/]+\/configurations$/, () => envelope({})],
  [
    'DELETE',
    /^\/client\/v4\/accounts\/[^/]+\/cfd_tunnel\/[^/]+$/,
    () => envelope({ id: TUNNEL.id }),
  ],
  ['GET', /^\/client\/v4\/accounts\/[^/]+\/cfd_tunnel\/[^/]+$/, () => envelope(TUNNEL)],
];

const server = createServer((req, res) => {
  // Drain the body (some handlers are POST/PUT/PATCH) but we don't inspect it.
  req.on('data', () => {});
  req.on('end', () => {
    const url = (req.url ?? '').split('?')[0];
    for (const [method, regex, handler] of routes) {
      if ((method === null || method === req.method) && regex.test(url)) {
        const payload = handler();
        if (process.env.CF_MOCK_DEBUG)
          console.error(`[cf-mock] ${req.method} ${url} -> ${payload}`);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(payload);
        return;
      }
    }
    // Unmatched: still return a CF-shaped failure so the app surfaces a clean
    // error instead of a parse failure, and log it for debugging new paths.
    console.error(`[cf-mock] unmatched ${req.method} ${url}`);
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({
        success: false,
        errors: [{ code: 7003, message: `cf-mock: no stub for ${req.method} ${url}` }],
        messages: [],
        result: null,
      }),
    );
  });
});

server.listen(PORT, () => {
  console.log(`cloudflare mock listening on http://127.0.0.1:${PORT}/client/v4/`);
});
