# runners/CLAUDE.md

Runners reconcile Patr resources (deployments + managed URLs) onto a backend. See root `CLAUDE.md` for workspace-wide rules, and the per-crate `CLAUDE.md` in `common/` and `docker/`.

## The model

- `runners/common` is the framework; a concrete runner (docker) is a thin binary that implements the single **`RunnerExecutor`** trait (`common/src/executor.rs`) and calls `Runner::<E>::init().await?.run().await`.
- State lives in a local **SQLite** DB (desired state); the executor reports actual state; the runner reconciles.
- The runner is headless: it always connects to the Patr API over WebSocket, and serves no HTTP API or UI of its own — see below.
- The runner is **its own identity**: config `apiToken` is its `patr_rn_…` runner token (Bearer to the API and WebSocket, Basic password to loki/mimir/openbao). Its permissions are fixed in the API (`api/src/models/permissions/runner.rs`): read across its workspace, `runner::{view,edit}` on itself. A new API call from the runner therefore needs `Runner` in the endpoint's `client_type` and a permission that map holds; widening the map is an access-control change. A rejected token or `RunnerAlreadyConnected` is logged with the `patr runner setup reconnect` fix and retried, never fatal.
- Architecture is a **ractor actor tree**: `RunnerSupervisor → ResourceSupervisor + WebSocketActor → DeploymentActor`. Deployments are actor-backed; managed URLs are reconciled statelessly. (This actor refactor is done, not pending.)

## Adding a runner

New crate depending on `common`, implement `RunnerExecutor`, `main = Runner::<E>::init().await?.run().await`, add a `config/runner.<name>.sample.json`, register in workspace members. Config filename is derived from the **binary name** at runtime (`runner_internal_name()`), so renaming the binary silently changes which config file is read.

Adding a new *resource type* (database, static site) is scaffolded for but not wired: `ResourceSupervisorMessage` carries a `resource_type` field that's currently `#[allow(dead_code)]`.

## `runners/kubernetes` — reference only, not built

Crate name is `controller`. It predates the common-library/actor design, does **not** depend on `common`, doesn't implement `RunnerExecutor`, and its `main` is effectively dead. It's `exclude`d from the workspace, so nothing builds it and its `workspace = true` dependency inheritance no longer resolves — it's kept purely as a reference for a future Kubernetes runner. **Don't extend it or wire it back up without rewriting it against `RunnerExecutor`.**

## Self-hosted operation

The runner's old standalone SelfHosted mode (its own auth, workspace HTTP API and embedded UI) is gone. Self-hosted operation runs the central `api` built without the `cloud` feature (`cargo build -p api --no-default-features`), with the runner as a pure executor behind it.

Both crates need the nightly toolchain (`never_type`).
