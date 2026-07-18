# api/CLAUDE.md

The backend binary. See the root `CLAUDE.md` for workspace-wide build/sqlx/style rules.

## One app, many hostnames

`api/` serves six logical hosts off a single axum app, dispatched by the `Host` header (`src/routes/mod.rs`): `api.` (REST API), `app.` (dashboard — `/api/*` re-mounts the API as `WebDashboard`, everything else reverse-proxies to `FRONTEND_URL`), `registry.` (OCI registry), `loki.` / `mimir.` (authenticated push proxies), `assets.`.

**In debug builds each host gets its own port** (`src/app.rs`): base `bind_address` = api, +1 app, +2 registry, +3 loki, +4 assets, +5 mimir. Hit `localhost:<base+N>` locally, not vhosts. Release dispatches all on one port by Host header.

api/ does **not** embed the frontend — it reverse-proxies to `FRONTEND_URL` (default `http://localhost:3030`).

The **self-hosted** build (`--no-default-features`) collapses the six-way `Host` fanout into a single base-domain path router (`/api`, `/mimir`, `/assets`, `/v2` for the registry, frontend fallback) — see the cloud/self-hosted section in root `CLAUDE.md`.

## Endpoints: declared in `models`, handled here

An endpoint's shape — path, method, request/response DTOs, `authentication`, `audit_log`, `#[preprocess(...)]` validation, RBAC permission — is declared with `macros::declare_api_endpoint!` in the **`models`** crate. `api/` only holds the **handler** and mounts it. Adding an endpoint = (1) declare it in `models`, (2) write the handler under `src/routes/<host>/...`, (3) `mount_*` it in that module's `setup_routes`.

- Mount via the `RouterExt` trait: `.mount_endpoint` (unauth), `.mount_auth_endpoint`, `.mount_registry_endpoint`.
- Handlers destructure `AuthenticatedAppRequest { request, database, redis, client_ip, user_data, state }` and return `Result<AppResponse<E>, ErrorType>`.
- **The layer stack owns the DB transaction** (`DataStoreConnectionLayer`): it auto-commits on `Ok`, auto-rolls-back on `Err`. Handlers never begin/commit a tx — just return `Result`.
- `mount_*` takes an `allowed_client_type`. If a client is `ApiToken` and the endpoint's `API_ALLOWED` is false, it's silently not mounted.

## Auth & caching

- Web dashboard sessions use **JWT**; API tokens are `patrv1.{refresh_token}.{login_id}` (parsed in `src/models/permissions/api_token.rs`, `src/utils/layers/registry/authenticator.rs`).
- **Redis** (`rustis`, `src/redis/`) is not just a cache — it holds the cached permission map per `login_id` (with multi-level revocation timestamps; validity `CACHED_PERMISSIONS_VALIDITY` = 2 days), rate-limit buckets (sorted sets), pub/sub for WebSocket log/metric streams, and operational caches. Key namespace lives in `src/redis/keys.rs`.

## Database & migrations

- Runtime DB access uses the compile-time-checked `query!` macro against `&mut *database`. Reusable helpers live in `src/db/{user,workspace,rbac}/*`.
- **Migrations** (`src/migrations/vX_Y_Z/mNNN_name.rs`, `#[macros::migration]`, auto-registered via `inventory`) **must use runtime `sqlx::query(...)`, never `query!`** — the schema is mid-change.
- Scaffold with `cargo new-migration <name>`. Apply with `cargo run --bin api -- --migrate`.
- **When you change the schema, write the migration in the same change** — the app runs on a live server; a missing migration breaks deploys.

## Registry

The OCI registry has its **own parallel stack** (`RegistryEndpoint` trait, `RegistryError`, streaming `RegistryResponse`, its own layers under `src/utils/layers/registry/`). Mount order matters to avoid path conflicts (see `src/routes/registry.patr.cloud/mod.rs`). It's under active rework — read the surrounding code, and be careful with conformance.

## Bindings

After renaming or changing any request/response type (in `models`), run `cargo bindings` or CI fails on stale `frontend/src/bindings`.

## Background jobs

apalis job queue in `src/worker/` (`WorkerTaskType` enum, Postgres-backed). Cron jobs registered in `worker/mod.rs::run`. Add a background task by extending `WorkerTaskType` / cron registration there.

## Tests (run from repo root)

- Integration: `just api test [filter]` — boots docker-compose (pg/redis/minio/loki/mimir), copies config, runs `--migrate`, then `cargo nextest run -p api --test integration-tests`.
- OCI conformance: `just api conformance`.
- (These recipes live in `api/tests/Justfile`, wired into the root `Justfile` as `mod api`.)
