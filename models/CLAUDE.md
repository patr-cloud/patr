# models/CLAUDE.md

Shared DTOs and types used by the API, frontend (via generated TS bindings), CLI, ingress worker, and runners. See root `CLAUDE.md` for workspace-wide rules.

## Structure

- `src/api/` mirrors the HTTP endpoint tree (one file per endpoint, a `mod.rs` per resource for shared DTOs).
- Endpoints are declared with `macros::declare_api_endpoint!` (grammar: doc comment, name, `METHOD "/path" { path }`, then optional `request_headers` / `authentication` / `query` / `request` / `response` / `audit_log`). Streaming endpoints use `declare_stream_endpoint!`. The macro generates the `Path`/`Query`/`RequestHeaders`/`Request`/`Response` structs and the `ApiEndpoint` impl.
- `src/rbac/` holds `Permission` and its sub-enums — the RBAC source of truth.
- `WithId<T>` / `OnlyId` flatten an `id: Uuid` onto a DTO via `#[serde(flatten)]` — the inner `T` must not carry its own id.

## TS bindings

- The endpoint macro auto-derives `ts_rs::TS` with `#[ts(export, rename_all = "camelCase")]` — generated request/response types export with no extra annotation.
- **Hand-written shared DTOs must opt in manually**: add `TS` to the derive list and `#[serde(rename_all = "camelCase")]`, matching the surrounding block. Field overrides use `#[ts(type = "...")]` / `#[ts(as = "...")]` (e.g. `OffsetDateTime` → `Date`).
- **Regenerate after ANY type change here**: `cargo bindings` (= `test -p models export_bindings`; `TS_RS_EXPORT_DIR = frontend/src/bindings`). **CI fails on stale bindings.** All bindings must be reachable from the `frontend/src/bindings/index.ts` barrel.

## Gotchas

- **This crate compiles to wasm** — the `ingress` Cloudflare Worker (wasm32) depends on `models`. Native-only deps (`sqlx`, native `uuid`) are gated behind `[target.'cfg(not(target_arch = "wasm32"))']` in `Cargo.toml`, and sqlx derives use `#[cfg_attr(not(target_arch = "wasm32"), ...)]`. **Don't add native-only dependencies unconditionally** — it breaks the ingress wasm build.
- **UUIDs are non-hyphenated** — `Display`/`Serialize`/sqlx all use `.simple()` (`src/utils/uuid.rs`). Use this crate's `Uuid`, never `uuid::Uuid` directly.
- Request fields carry `#[preprocess(...)]` (`trim`, `lowercase`, `regex = ...`, or `none`); regexes live in `utils::constants`.
- Needs nightly (`#![feature(adt_const_params)]`). Private items need doc comments (except `api/mod.rs`, which allows the missing-docs lint).
