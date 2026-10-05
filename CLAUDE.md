# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Patr is a DevOps automation platform. Rust workspace (backend, CLI, runners, Cloudflare Worker) + a SolidJS frontend + a Playwright e2e suite. Subdirectories have their own `CLAUDE.md` with details — this file is the workspace-wide baseline.

**Keep these docs current.** When a change is significant enough to invalidate something written here or in a subdirectory's `CLAUDE.md`, update that file in the same change.

## Workspace layout

- `api/` — backend binary; serves the REST API, web dashboard, OCI registry, and log/metric proxies off one axum app.
- `cli/` — the `patr` CLI (`cargo patr`).
- `ingress/` — Cloudflare Worker (wasm32 `cdylib`) that proxies requests to the right cluster. Depends on `models`.
- `macros/` — proc macros (`declare_api_endpoint!`, `query!`, `migration`).
- `models/` — DTOs and shared types; source of the frontend's generated TS bindings.
- `runners/{common,docker}` — deployment runners. `common` is the framework; `docker` is the live runner. `runners/kubernetes` is kept as a reference implementation only — it's excluded from the workspace and never built.
- `frontend/` — SolidJS dashboard (pnpm). `e2e/` — Playwright suite (pnpm).

## Cloud vs self-hosted

The same codebase builds two flavors. The **cloud** build is the public `patr.cloud` SaaS; the **self-hosted** build is what an operator runs on their own infra. Cloud-only surfaces (Cloudflare, GitHub OAuth, Turnstile, IPInfo, managed tunnels) are gated at compile/build time so self-hosted binaries never link or ship them.

- **Backend:** `cloud` Cargo feature on `api` (propagated to `models`), default-on. Self-hosted = `cargo build -p api --no-default-features`. Gating is `#[cfg(feature = "cloud")]` on items and `cfg_if!` for divergent bodies. What's gated: the `Host`-header fanout to seven subdomains (cloud) vs a single base-domain path router (self-hosted, `/api`, `/mimir`, `/assets`, `/openbao`, `/v2` for the registry, frontend fallback); GitHub OAuth + Turnstile in the auth routes; the domain-verify / managed-URL / cleanup cron workers (self-hosted runs only the email worker); and the Cloudflare write paths in `runner`/`managed_url`/`deployment`. `AppConfig` fields `primary_hosted_domain`/`cloudflare`/`ipinfo`/`social_login` are cloud-only — `config/api.sample.json` is self-hosted-valid, cloud adds those blocks. Secrets (workspace `secret` routes + the `openbao.` read proxy) are **not** gated: they build in both flavors, so `open_bao` is required in both and a self-hosted deploy has to run an OpenBao instance. `cf_turnstile_token` stays a plain `String` (not `Option`); the server's validation is `cfg`-gated, so self-hosted accepts any value.
- **Frontend:** `VITE_CLOUD_MODE` (default off). `pnpm build` = self-hosted bundle; `VITE_CLOUD_MODE=true pnpm build` = cloud. See `frontend/CLAUDE.md`.
- **Builds:** API cloud `cargo build --release -p api --locked`; API self-hosted add `--no-default-features`; CLI `cargo build --release -p cli --locked`.

## Build & test

- **Build/check individual packages, never `--workspace`.** The workspace build fails (missing generated dirs — `runners/common` RustEmbeds `frontend/.output/public`), and per-package builds are also what populate the `.sqlx/` cache. Use `cargo check -p api`, `cargo build -p api`, etc.
- **`cargo build -p api` and `cargo build -p docker` must be separate invocations.** Compiling both at once unifies `common`'s features and fails to build. Run them one after the other, never `cargo build -p api -p docker`.
- **`cargo check` skips test targets.** Verify tests compile with `cargo test -p <pkg> --no-run`.
- **Clippy per-package** (`cargo clippy -p api`), not workspace-wide — other crates have pre-existing lint errors.
- Toolchain is pinned **nightly** (`rust-toolchain.toml`); plain `cargo` is already nightly — don't add `+nightly`.

## SQLx (offline)

- Builds run with **`SQLX_OFFLINE=true`** and compile against the committed `.sqlx/`. There is no committed root `.env`; when building locally without a live DB, prefix `SQLX_OFFLINE=true`.
- After changing any `query!`, regenerate the offline cache with **`just prepare`** — it prepares both feature sets (cloud + `--no-default-features`) and merges them into one `.sqlx/` (hash-named files collapse shared queries and let feature-only ones coexist). Commit the `.sqlx/` change alongside the query/schema change that prompted it, or CI/offline builds break.

## Conventions

- **Tabs, not spaces**, everywhere (`.rustfmt.toml` `hard_tabs`, `max_width = 100`). Frontend and e2e also use tabs (see their `CLAUDE.md`).
- **Doc comments are near-mandatory** — `missing_docs` and `clippy::missing_docs_in_private_items` are warnings, so even private items need docs.
- **UUIDs are non-hyphenated everywhere** (DB, API, URLs, log labels). Use the crate's own `Uuid` (`models::prelude::Uuid`), never `uuid::Uuid` directly.
- `unsafe_code` is forbidden.
- Prefer **turbofish / suffix syntax** over `let` bindings with explicit types — `.collect::<Vec<_>>()`, `.parse::<u32>()`, `0u32`, not `let x: Vec<_> = …`.
- Prefix unused variables with `_`.

## Cargo aliases (`.cargo/config.toml`)

- `cargo api` — run the backend (dashboard on `http://localhost:3001`).
- `cargo docker` — run the Docker runner.
- `cargo patr` — run the CLI.
- `cargo bindings` — raw ts-rs export (`test -p models export_bindings`). **Don't run it directly — use `just bindings`**, which runs it *and* rebuilds the `frontend/src/bindings/index.ts` barrel that ts-rs doesn't manage. **CI fails if bindings are stale.**
- `cargo new-migration <name>` — scaffold a DB migration.

## Config

Copy `config/api.sample.json` → `config/api.json` (and `config/runner.docker.sample.json` → `config/runner.docker.json` for the runner). Any config value is env-overridable with `PATR__SECTION__KEY` (double underscore = nesting).

## Git

- Branches: `feature/<name>`, `fix/<name>`, `refactor/<name>`.
- Flow: `develop` → `staging` → `master`. `develop` auto-deploys to alpha.
- Commits: sentence case, lazy in tone but grammatically correct; small change = one short line. No `Co-Authored-By`, no headers, no emoji.

## Area agents

Each part of Patr has an agent in `.claude/agents/` that owns it: `api`, `access-control`, `security`, `runners`, `frontend`, `cli`, `iaac`, `platform` and `user-communication`. A tenth agent, `meta`, keeps the agents, skills and `CLAUDE.md` files in sync with the code. Each area agent catches what a feature leaves out of its area, including work no diff points to, and implements its area's part of a plan.

- **Scoping is a gate in plan mode.** `/scope-feature-coverage` asks every area agent what a feature needs and folds the answers into the plan. It's expensive, so **only the user runs it**; never run it yourself. When a plan adds or changes user-facing surface (a resource, field, operation, principal or behaviour):
  - **Ask before writing the execution section,** with `AskUserQuestion`. Offer "Run `/scope-feature-coverage` now (Recommended)" and "Skip", and say why the agents matter for this feature. Mentioning it inside another message doesn't count as asking.
  - **Don't call `ExitPlanMode` until the user has answered.** If they run it, fold the results into the plan first.
  - **Scope the whole feature,** never just part of it or a later PR.
  
  Bug fixes, refactors and internal changes don't need it.
- **Plans carry everything.** A plan's execution section has:
  - a stage table giving the stage, agent and task for each piece of work;
  - where bindings get regenerated;
  - the verification commands;
  - the commits.

  A plan that names an agent for a task is the go-ahead to hand that task over without asking again. Agents work in the current checkout and never commit.
- **Stages:**
  1. **Contract.** `api`, then `access-control`. Run `just bindings` at the end of the stage.
  2. **Consumers, in parallel.** `runners`, `frontend`, `user-communication`, and `cli` then `iaac`. Those last two share a crate, so they run in that order.
  3. **Cross-cutting.** `platform` (gating, config and deploy notes), then `security` (review only, unless the plan assigns a fix), then a full build and test, then `meta`.

  Agents with nothing to do in a feature drop out of their stage.
- **Backlogs.** Areas that aren't fully built yet keep untracked backlogs at the repo root: `DOCS_TODO.md`, `IAAC_TODO.md` and `CLI_TODO.md`. Gaps a plan defers in those areas go there. Never commit them. Built areas have no backlog; their gaps go into the plan.
- **Migrations.** Every schema change goes through the `migrations` skill.
