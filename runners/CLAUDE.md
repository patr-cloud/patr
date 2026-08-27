# runners/CLAUDE.md

Runners reconcile Patr resources (deployments + managed URLs) onto a backend. See root `CLAUDE.md` for workspace-wide rules, and the per-crate `CLAUDE.md` in `common/` and `docker/`.

## The model

- `runners/common` is the framework; a concrete runner (docker) is a thin binary that implements the single **`RunnerExecutor`** trait (`common/src/executor.rs`) and calls `Runner::<E>::init().await?.run().await`.
- State lives in a local **SQLite** DB (desired state); the executor reports actual state; the runner reconciles.
- Two modes: **Managed** (connects to the Patr API over WebSocket) and **SelfHosted** (standalone, serves its own auth + workspace HTTP API + embedded UI). SelfHosted is legacy and being removed — see below.
- Architecture is a **ractor actor tree**: `RunnerSupervisor → ResourceSupervisor + WebSocketActor → DeploymentActor`. Deployments are actor-backed; managed URLs are reconciled statelessly. (This actor refactor is done, not pending.)

## Adding a runner

New crate depending on `common`, implement `RunnerExecutor`, `main = Runner::<E>::init().await?.run().await`, add a `config/runner.<name>.sample.json`, register in workspace members. Config filename is derived from the **binary name** at runtime (`runner_internal_name()`), so renaming the binary silently changes which config file is read.

Adding a new *resource type* (database, static site) is scaffolded for but not wired: `ResourceSupervisorMessage` carries a `resource_type` field that's currently `#[allow(dead_code)]`.

## `runners/kubernetes` — reference only, not built

Crate name is `controller`. It predates the common-library/actor design, does **not** depend on `common`, doesn't implement `RunnerExecutor`, and its `main` is effectively dead. It's `exclude`d from the workspace, so nothing builds it and its `workspace = true` dependency inheritance no longer resolves — it's kept purely as a reference for a future Kubernetes runner. **Don't extend it or wire it back up without rewriting it against `RunnerExecutor`.**

## SelfHosted mode → headless executor

SelfHosted mode currently runs standalone: its own auth + workspace HTTP API and an **embedded frontend** (this is the `frontend/.output/public` RustEmbed in `runners/common` — see its `CLAUDE.md`). **That whole surface is being removed.** The runner is becoming a **headless executor** — no embedded UI, no self-hosted API. Self-hosted operation instead runs the central `api` built without the `cloud` feature (`cargo build -p api --no-default-features`), with the runner as a pure executor behind it.

So: **don't invest in the runner's frontend/API surface.** When a shared `models` change breaks its exhaustive destructuring, take the **minimal compile fix** (`field: _,`, `cfg_if` to `FeatureNotSupported`) — never reimplement feature parity.

Both crates need the nightly toolchain (`impl_trait_in_assoc_type`, `never_type`).
