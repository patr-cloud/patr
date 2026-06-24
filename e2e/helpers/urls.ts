// Shared base URLs for the e2e suite. All three are bound to localhost ports
// the Justfile brings up — keep this file in sync with `e2e/Justfile`'s
// `VITE_BASE_URL` and `VINXI_PORT` exports.
//
// Pick the right one for what you're doing:
//
// - DASHBOARD_URL  → app.patr.cloud entrypoint. The browser SPA loads from
//                    here. /api/** is proxied to the API with cookie auth
//                    (ClientType::WebDashboard). Use this for anything that
//                    mirrors how a real browser session talks to the backend
//                    — programmatic logins, cookie reads, integration-style
//                    fetch() calls from specs.
//
// - API_DIRECT_URL → api.patr.cloud entrypoint. Direct REST surface with
//                    Bearer auth (ClientType::ApiToken). API tokens MUST
//                    target this URL — the /api proxy on DASHBOARD_URL
//                    refuses Bearer tokens with 400 because it expects the
//                    authState cookie.
//
// - VINXI_DEV_URL  → the underlying Vinxi dev server. Almost nothing should
//                    talk to this directly; it's the upstream that the
//                    DASHBOARD_URL proxy fronts. The two specs that use it
//                    are reading cookies set by a route that's only reachable
//                    via Vinxi during dev mode (no Caddy in between).

export const DASHBOARD_URL = 'http://localhost:3001';
export const API_DIRECT_URL = 'http://localhost:3000';
export const VINXI_DEV_URL = 'http://localhost:13030';

// The runner pulls Patr-registry images from the hardcoded `registry.patr.cloud`
// (no code override). Inside the DinD we make that name resolve to the host's
// API registry via a port-bridge (see dind.ts): `--add-host registry.patr.cloud
// :127.0.0.1` plus a socat forward `:443 → host.docker.internal:3002`. Pushes go
// through the same DinD daemon, so this one host value is used everywhere.
export const REGISTRY_HOST = 'registry.patr.cloud';

// The host API listener ports the in-DinD socat bridges forward to (reached via
// host.docker.internal): the OCI registry (API port + 2) and the API itself
// (for the docker-login token realm, http://localhost:3000/auth/docker-login).
export const HOST_REGISTRY_PORT = 3002;
export const HOST_API_PORT = 3000;
