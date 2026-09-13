# runners/docker/CLAUDE.md

The live runner: implements `RunnerExecutor` over **Docker Swarm** via `bollard`. See `runners/CLAUDE.md` and `runners/common/CLAUDE.md` for the shared model.

## It uses Swarm services, not plain containers

- A deployment → a Swarm **service** `patr-{deployment_id}` (replicas = `min_horizontal_scale`).
- Config mounts, ingress routing, tunnel tokens, alloy config → Swarm **configs** (Swarm's "file into a container" mechanism).
- Volumes → named Docker **volumes** `patr-{deployment_id}-{sha256(path)[:16]}`, one per path, labelled `managed-by=patr` + `patr.deploymentId`. Name is path-derived (never an ordinal) so remove-then-re-add finds the same data. **Detaching never deletes a volume** — only `deployment::delete` does, via a label sweep with a short 409 retry (Swarm reaps exited tasks async). `stop_deployment` tears down the service and keeps volumes; that split is what makes Stop safe. Volumes are node-local; all managed runners are single-node today.
- Image resolution: Patr-registry deployments carry only a `repository_id`, so it calls the API (`GetContainerRepositoryInfo`) to resolve the name; digest-pinned when a live digest is set. Patr-registry pulls use `patr` / api-token creds; external registries pull anonymously.

## Two non-obvious patterns

- **`ingress_lock` mutex** serializes the read-modify-write of the single shared `patr-ingress` Caddy service. Swarm updates use an optimistic version index — every `update_service` must first read `service.version.index` and pass it back, or concurrent actors collide with "update out of sequence".
- **Content-hash config naming**: Swarm configs are immutable, so `utils::update_config` names them `{base}-{sha256(data)[:16]}` — same hash reuses, changed hash creates-new-then-deletes-old. Docker's config API expects base64-encoded data and bollard 0.21 does **not** encode for you — the code base64-encodes on write / decodes on read manually. Config lists are built into a `BTreeMap` for deterministic order, else every update looks "changed" and triggers a Caddy hot-reload loop.

## Ingress / exposure

Ingress is a Caddy service (`patr-ingress`), always deployed. `PUBLIC` publishes 80/443 with ACME (LetsEncrypt **staging** in debug, production in release); `PRIVATE` publishes nothing and runs a Cloudflare tunnel (**managed mode only** — self-hosted + tunnel returns `Unsupported`). Alloy log/metric scraping is managed-mode only.

## Known holes (verified)

- **Secret env vars `todo!()` → runtime panic** (`src/deployment.rs`): a deployment with `EnvironmentVariableValue::Secret` will panic at reconcile. Not yet implemented.
- Paused deployments and `machine_type` are ignored. Swarm supports one healthcheck, so `liveness_probe` wins over `startup_probe`.
- **Update order depends on volumes**: `start-first` (zero-downtime) for stateless deployments, `stop-first` when volumes are present so the old task releases them before the new one mounts them.
- **`enableIpv6` (default true) must be set false** on hosts whose Swarm has no IPv6 address pool, or every task fails to get an address.

## Config / lifecycle / build

- Config `config/runner.docker.json` (copy from `config/runner.docker.sample.json` — the sample omits the docker-only keys, which all have serde defaults). Everything Patr-created is labeled `managed-by=patr`; reconciliation is label-filter driven.
- Config deletion **order** matters: unmount a config from the service (rebuild + update ingress without it) **before** deleting it, else Docker rejects "config in use".
- **Build standalone**: `cargo build -p docker` — never in the same invocation as `api` (feature unification breaks it). Run with `cargo docker`.
