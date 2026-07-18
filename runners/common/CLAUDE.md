# runners/common/CLAUDE.md

The shared runner framework: the `RunnerExecutor` trait, `Runner<E>`, the actor tree, upstream WebSocket, local SQLite state, reconciliation, and HTTP routes. See `runners/CLAUDE.md` for the cross-cutting picture.

## Implementing an executor

- Implement `RunnerExecutor` (`src/executor.rs`). Two-phase: `initialize()` builds heavy shared state once (Docker client, etc.) into `InitializedState`; **`new()` is called per reconcile pass and per actor spawn, so keep it cheap** (clone the handle).
- `list_running_*` **must return sorted streams** — reconciliation is a sorted merge of SQLite vs running actors.
- Returning `Err(RunnerError)` is the **retry signal**: on failure, an actor reports `Errored` upstream then returns `Err` to kill itself; the supervisor respawns with exponential backoff (1s → 5min).
- Import surface is the prelude: `use common::prelude::*` (trait, `Runner`, `RunnerError`, config, `DatabaseType = sqlx::Sqlite`, `sqlx::{query, Row}`).

## Gotchas

- **Embeds `../../frontend/.output/public`** (`RustEmbed` in `src/routes/mod.rs`, the SelfHosted-mode local UI) — that dir must exist or **`cargo build -p common` fails**. Build the frontend first, or `mkdir -p frontend/.output/public`. **This embed (and the whole SelfHosted API/UI surface) is legacy and being removed** — the runner is moving to a headless executor (see `runners/CLAUDE.md`). Don't build on it; minimal compile fixes only.
- **Migrations use runtime `query()`, never the `query!` macro.** Not (only) because the schema changes — sqlx 0.8 can't support two databases (Postgres for `api` + SQLite for the runners) in one project with the compile-time macros. (Expected to change in sqlx 0.9.) Migrations self-register via `inventory`, and the DDL + bookkeeping insert must happen in **one transaction** (partial DDL leaves `*_new` temp tables that wedge retries).
- **Tests:** `cargo nextest run --package common`. `managed_mode::` tests are forced single-threaded (`.config/nextest.toml`) — they share one real HTTP test server on **fixed port 3000**. Non-managed tests get isolated temp SQLite and run in parallel.
- **FullResync is destructive-in-transaction** (`src/actors/websocket.rs`): it DELETEs all deployment/managed-url tables and repopulates from the API in one transaction (rolls back on error). Runs on every WS (re)connect and on a timer — executor ops must tolerate this churn. Table-clear order matters (managed URLs before deployments — FK).
- `db::initialize` (migrations) runs **before** `E::initialize`. `run()` returns `!`. rustls' crypto provider is installed manually in init because ring + aws-lc-rs are both linked.
