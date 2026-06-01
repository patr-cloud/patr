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
