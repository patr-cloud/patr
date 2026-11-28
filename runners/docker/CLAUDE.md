# runners/docker/CLAUDE.md

The live runner: implements `RunnerExecutor` over **Docker Swarm** via `bollard`. See `runners/CLAUDE.md` and `runners/common/CLAUDE.md` for the shared model.

## It uses Swarm services, not plain containers

- A deployment → a Swarm **service** `patr-{deployment_id}` (replicas = `min_horizontal_scale`).
- Config mounts, ingress routing, tunnel tokens, alloy config → Swarm **configs** (Swarm's "file into a container" mechanism).
- Volumes → named Docker **volumes** `patr-{deployment_id}-{sha256(path)[:16]}`, one per path. Name is path-derived (never an ordinal) so remove-then-re-add finds the same data. A stopped deployment still expects its volumes, so Stop keeps them, and a volume whose path was removed is kept while its deployment exists. Once the deployment is deleted or moved, the volume finalizer confirms that with the API before the volume is deleted. Volumes are node-local; all managed runners are single-node today.
- Image resolution: Patr-registry deployments carry only a `repository_id`, so it calls the API (`GetContainerRepositoryInfo`) to resolve the name; digest-pinned when a live digest is set. Patr-registry pulls use `patr` / the runner's token; external registries pull anonymously.
- Secret env vars: each `EnvironmentVariableValue::Secret` is resolved in `apply` via `secrets::get_secret_value` → `openbao.patr.cloud` (OpenBao's KV v2 API behind Basic `{runner_id}:{runner token}` auth), then baked into the service env. Values are never logged or stored in SQLite. The spec carries each secret's `last_updated` (from `SecretUpdated` pushes and every FullResync) instead of its value, so a rotation changes the spec hash and Swarm rolls the new env.
- **Objects and specs** (`runner.rs`): `DockerObjectId` is a config, network, service or volume by name; `DockerSpec` is what it's built from, and is never `Debug`-printed (configs hold file contents, the Alloy config holds the API token). Configs, networks and volumes are create-if-missing; services are created or updated, retrying once on "update out of sequence". The deployment spec's JSON is pinned by a golden test (`tests/expected.rs`), because it's hashed.

## Two non-obvious patterns

- **Only `System` touches the shared `patr-ingress` Caddy service**, so no lock is needed. It mounts the routes (deployment default URLs and managed URLs) that exist when it's applied: they belong to other owners, so they aren't graph edges, and a route that doesn't exist yet stamps a hash that won't match, so the next reconcile retries. It updates `start-first`, so a route change never drops traffic.
- **Content-hash config naming**: Swarm configs are immutable, so configs are named `{base}-{sha256(data)[:16]}` (`utils::config_name`). New content gets a new config, the service referencing it is re-applied, and the old config is reaped once nothing uses it. Docker's config API expects base64-encoded data and bollard 0.21 does **not** encode for you — the code base64-encodes on write / decodes on read manually.

## Ingress / exposure

Ingress is a Caddy service (`patr-ingress`), always deployed. `PUBLIC` publishes 80/443 with ACME (LetsEncrypt **staging** in debug, production in release); `PRIVATE` publishes nothing and runs a Cloudflare tunnel, once the WebSocket loop has stored its token in the `patr-tunnel-token` config (on every connect). Alloy log/metric scraping always runs. In development Alloy pushes to `host.docker.internal`, which its service maps to the bridge gateway so a Docker-in-Docker runner (the e2e suite) reaches the host. Swarm's `ContainerSpec.hosts` takes `IP hostname` (hosts(5)); the `--add-host` form `hostname:IP` is one field, and Docker silently drops it.

Every route imports `(patr_errors)` from `Caddyfile.base`, which serves `assets/error-pages` for Caddy's own errors: a stopped deployment's routes answer `error 503` (the "deployment stopped" page) and a dial failure serves the connection-error page. An app's own error response is never replaced. Managed URL hosts and paths are checked before they're written into a site address, and an unsafe one fails only that managed URL.

## Known holes (verified)

- Paused deployments and `machine_type` are ignored. Swarm supports one healthcheck, so `liveness_probe` wins over `startup_probe`.
- **Update order depends on volumes**: `start-first` (zero-downtime) for stateless deployments, `stop-first` when volumes are present so the old task releases them before the new one mounts them.
- **`enableIpv6` (default true) must be set false** on hosts whose Swarm has no IPv6 address pool, or every task fails to get an address.

## Config / lifecycle / build

- Config `config/runner.docker.json` (copy from `config/runner.docker.sample.json` — the sample omits the docker-only keys, which all have serde defaults). Everything Patr-created is labeled `managed-by=patr`; reconciliation is label-filter driven.
- A config still mounted by a service can't be deleted ("config in use"), which is why reaping skips objects `in_use` and retries on a later reconcile.
- **Build standalone**: `cargo build -p docker` — never in the same invocation as `api` (feature unification breaks it). Run with `cargo docker`.
