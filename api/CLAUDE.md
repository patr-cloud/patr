# api/CLAUDE.md

The backend binary. See the root `CLAUDE.md` for workspace-wide build/sqlx/style rules.

## One app, many hostnames

`api/` serves seven logical hosts off a single axum app, dispatched by the `Host` header (`src/routes/mod.rs`): `api.` (REST API), `app.` (dashboard — `/api/*` re-mounts the API for `WebLogin`, everything else reverse-proxies to `FRONTEND_URL`), `registry.` (OCI registry), `loki.` / `mimir.` (authenticated push proxies), `assets.`, `openbao.` (authenticated OpenBao read proxy — runners only, see below).

**In debug builds each host gets its own port** (`src/app.rs`): base `bind_address` = api, +1 app, +2 registry, +3 loki, +4 assets, +5 mimir, +6 openbao. Hit `localhost:<base+N>` locally, not vhosts. Release dispatches all on one port by Host header.

api/ does **not** embed the frontend — it reverse-proxies to `FRONTEND_URL` (default `http://localhost:3030`).

**`openbao.` is the runner's read path for secret values.** It mirrors OpenBao's own KV v2 API (`GET /v1/secret/data/{workspace_id}/{secret_id}`) and streams OpenBao's response back untouched, so any OpenBao-compatible client can read it — point a client's address at the host (or at `{base}/openbao` self-hosted) and its own `/v1/...` paths line up. The official Go client and `bao` CLI join a path prefix onto the request path, so the self-hosted form works; `vaultrs` does not, and only works against the cloud host. Auth follows the loki/mimir proxies: Basic `{runner_id}:{api_token}`, `Runner::Execute` on that runner, and the secret must live in the runner's workspace. Everything else about secrets (create/update/delete, metadata reads) stays on `api.`, which talks to OpenBao server-side. `secret.last_updated` tracks the **value**: a rotation bumps it and publishes `SecretUpdated` to every runner with a deployment referencing the secret; a rename does neither.

The **self-hosted** build (`--no-default-features`) collapses the seven-way `Host` fanout into a single base-domain path router (`/api`, `/mimir`, `/assets`, `/openbao`, `/v2` for the registry, frontend fallback) — see the cloud/self-hosted section in root `CLAUDE.md`.

## Endpoints: declared in `models`, handled here

An endpoint's shape — path, method, request/response DTOs, `authentication`, `audit_log`, `#[preprocess(...)]` validation, RBAC permission — is declared with `macros::declare_api_endpoint!` in the **`models`** crate. `api/` only holds the **handler** and mounts it. Adding an endpoint = (1) declare it in `models`, (2) write the handler under `src/routes/<host>/...`, (3) `mount_*` it in that module's `setup_routes`.

- Mount via the `RouterExt` trait: `.mount_endpoint` (unauth), `.mount_auth_endpoint`, `.mount_registry_endpoint`.
- Handlers destructure `AuthenticatedAppRequest { request, database, redis, client_ip, actor_data, state }` and return `Result<AppResponse<E>, ErrorType>`.
- **The layer stack owns the DB transaction** (`DataStoreConnectionLayer`): it auto-commits on `Ok`, auto-rolls-back on `Err`. Handlers never begin/commit a tx — just return `Result`.
- `mount_*` takes the host's client types. An endpoint whose `client_type` list shares none of them is silently not mounted on that host, and an authenticated endpoint only accepts the kinds in the overlap.
- **Handlers must check the id is their kind of resource.** `ResourcePermissionAuthenticator` only proves the path id is *some* live resource in the workspace and that the caller holds the permission on it — not that it's a secret, a volume, etc. So a handler for a typed id must confirm its own row exists before doing anything else, above all before soft-deleting the shared `resource` row. Otherwise e.g. `DELETE /secret/{deployment_id}` soft-deletes the deployment. Check the typed `DELETE`'s `rows_affected()` (see `delete_secret`, `delete_role`) or `fetch_optional(…).ok_or(ErrorType::ResourceDoesNotExist)` on the typed `SELECT` (see `delete_deployment`). **Every new resource type's handlers need this**, and `api/tests/api/workspace/rbac/resource_type.rs` should get a wrong-type case for its delete route.

## Auth & caching

- Web dashboard sessions use **JWT**. User API tokens are `patr_at_…` and service account tokens `patr_sa_…`: a kind prefix, 32 random base62 characters and a 6-character base62 CRC32 of the two (checked before any I/O, so a typo or a lookalike never reaches a lookup), issued by `permissions::generate_{api,service_account}_token` and stored as the SHA-256 of the whole token in `token_hash` (unique-indexed) and looked up by it. Every bearer token goes through `permissions::authenticate` (`src/models/permissions/mod.rs`), which only parses the kinds the route accepts: a JWT if it takes web logins, else the opaque prefix, which alone says the kind and is rejected before any I/O if the route doesn't take it. The cache (`ActorAuthDataCache`) is keyed by the login ID for JWTs and by the token hash for opaque tokens, so a hit does no DB work and, for tokens, finding the entry is itself the secret check. JWT claims and an API token's `allowed_ips` are re-checked per request.
- **Cache invalidation is by stamp, not by mutation.** Anything that changes what an entry would contain calls `permissions::mark_{login,token,actor,workspace,all}_stale`, which writes *now* to that scope's `*_cache_stale_since` key; an entry older than a stamp covering it is a miss. `mark_login_stale` (web logins) and `mark_token_stale` (opaque tokens, given the hash the handler reads back from the row — the old one on regenerate) also delete the entry. Entries and stamps share `CACHED_PERMISSIONS_VALIDITY` (2 days), so Redis **must run `maxmemory-policy noeviction`** — an evicted stamp would resurrect stale entries. Add a bump whenever you write a handler that changes credentials, roles, or membership.
- **Redis** (`rustis`, `src/redis/`) also holds rate-limit buckets (sorted sets), pub/sub for WebSocket log/metric streams, and operational caches. Key namespace lives in `src/redis/keys.rs`.

## Database & migrations

- Runtime DB access uses the compile-time-checked `query!` macro against `&mut *database`. Reusable helpers live in `src/db/{user,workspace,rbac}/*`.
- **Migrations** (`src/migrations/vX_Y_Z/mNNN_name.rs`, `#[macros::migration]`, auto-registered via `inventory`) **must use runtime `sqlx::query(...)`, never `query!`** — the schema is mid-change.
- Scaffold with `cargo new-migration <name>`. Apply with `cargo run --bin api -- --migrate`.
- **When you change the schema, write the migration in the same change** — the app runs on a live server; a missing migration breaks deploys.

## Registry

The OCI registry has its **own parallel stack** (`RegistryEndpoint` trait, `RegistryError`, streaming `RegistryResponse`, its own layers under `src/utils/layers/registry/`). Mount order matters to avoid path conflicts (see `src/routes/registry.patr.cloud/mod.rs`). It's under active rework — read the surrounding code, and be careful with conformance.

## Bindings

After renaming or changing any request/response type (in `models`), run `just bindings` or CI fails on stale `frontend/src/bindings`. Not bare `cargo bindings` — that skips the `index.ts` barrel rebuild.

## Background jobs

apalis job queue in `src/worker/` (`WorkerTaskType` enum, Postgres-backed). Cron jobs registered in `worker/mod.rs::run`. Add a background task by extending `WorkerTaskType` / cron registration there.

## Tests (run from repo root)

- Integration: `just api test [filter]` — boots docker-compose (pg/redis/rustfs/loki/mimir), copies config, runs `--migrate`, then `cargo nextest run -p api --test integration-tests`.
- OCI conformance: `just api conformance`.
- (These recipes live in `api/tests/Justfile`, wired into the root `Justfile` as `mod api`.)
