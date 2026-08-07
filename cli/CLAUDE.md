# cli/CLAUDE.md

The `patr` CLI — talks to the Patr API and manages on-host runners (systemd-installed Docker/Kubernetes workers). Binary `patr`, crate `cli`, entry `src/main.rs`. Run with `cargo patr`. See root `CLAUDE.md` for workspace-wide rules.

## Layout

- `commands/` — one module per command. Top-level commands (`login`, `logout`, `apply`, `upgrade`, `uninstall`, …) sit directly under it; workspace-scoped ones under `commands/workspaced/{workspace,deployment,container_registry,runner}/`.
- `utils/` — anything reused across ≥2 commands, re-exported through `prelude.rs`. `storage.rs` (`AppState` + disk IO), `client.rs` (`make_request` — the single HTTP entry point), `authenticator.rs`, `ext_trait.rs`. Single-use helpers stay co-located with their command.

## Command anatomy

Every command module has a `pub struct Args` (deriving `clap::Args`; empty if none) and `pub(super) async fn execute(args, global_args, state) -> Result<CommandOutput, AppError>`, plus an entry in the parent `mod.rs`'s `Subcommand` enum + dispatcher. Subcommands are `#[command(rename_all = "kebab-case")]`; add aliases liberally (`alias = "ls"`). Commands with no `Args` drop that parameter; runner subcommands that don't touch the API drop `global_args`/`state` — don't pad signatures.

## Output — `CommandOutput`, never `json!`

`execute` returns a `CommandOutput` built via `TypedBuilder` with a `.text(...)` (shown in default `OutputType::Text`, via `comfy_table` for tables) and a `.json(...)` (for `-o json` / `-o pretty-json`).

**Do not use `serde_json::json!`.** For JSON output, define a small local `#[derive(Serialize)]` struct (or reuse an API response type) and call `.to_json_value()` (from the `ToJsonValue` prelude trait) — a named struct is the schema; `json!` drifts. Nothing to return → `ApiSuccessResponseBody::empty().to_json_value()`.

## State

`AppState` is persisted to `~/.local/share/patr/cli/config.json` (`utils::storage`). Two fields: `target_channel: Channel` and `auth: AuthState` (an `#[serde(untagged)]` enum — `LoggedIn { token, current_workspace }` or `LoggedOut`). **The nested-enum shape is deliberate** — `current_workspace` can't exist without a `token`, so it's unrepresentable as two optional fields. To mutate: `load()` the full state, modify `state.auth`, `state.save()` — don't build a fresh `AppState` (you'd drop `target_channel`). `load()` falls back to `Default` on any error.

## Errors

Single `AppError` enum (`error.rs`), returned from every `execute`. Network errors convert via `From<ReqwestError>`, API errors via `From<ApiErrorResponse>`. `main` serializes errors through the requested `OutputType` — **don't print errors yourself, just return them.** Don't prepend "Error:" (callers add it). `AppError::RunnerError(String)` is the catch-all for shelled-out lifecycle failures (systemd, extraction, binary swap).

## API client

`utils::client::make_request` is the **only** way to hit the API — pass a typed `ApiRequest<E>` where `E: ApiEndpoint` (from `models`). It handles path/query encoding, user-agent, auth injection, deserialization. The reqwest client is a `OnceLock<Client>` — reuse it. `constants::{API_BASE_URL, FRONTEND_BASE_URL}` switch on `cfg!(debug_assertions)`.

## TTY / systemd / self-lifecycle

- Prompts use `inquire`, wrapped with `.expect_tty("…")` (`TtyExpectable`) so non-TTY stdin exits cleanly instead of panicking. Destructive commands also check `stdin().is_terminal()` and hint `-y`.
- Runner `service {install,uninstall,status}` share `run_systemctl` (strict; `sudo` when non-root) and `sudo_spawn_error`. `uninstall`'s `stop`/`disable` are deliberately lenient (not-running shouldn't fail); `daemon-reload` + unit removal stay strict. Always check `/run/systemd/system` exists first.
- `upgrade`/`uninstall` are gated off when built `--features package-managed` (Homebrew/distro builds) — the CLI refuses to touch its own install, no path-sniffing.

## Library > shell

When a crate exists for the job, use it: `sha2` (not `sha256sum`), `tar`+`flate2`/`zip` (not `tar`/`unzip`), `tempfile` (not hand-rolled temp), `self_replace` (not `mv`/`rm`), `reqwest` (not `curl`). Shell out only for OS-owned things (`systemctl`, `sudo`, `docker`). Pre-binary install logic lives in `assets/cli/install.sh`.

## Build metadata

CI bakes four vars via `build.rs` (`PATR_BUILD_{VERSION,CHANNEL,SHA,DATE}`) using `cargo:rustc-env`. Local `cargo build` leaves them unset → binary reports `<version>-dev`. `Channel::BUILD` is a compile-time const from `PATR_BUILD_CHANNEL` (falls back to `Alpha`).

## `patr apply` and the IaaC schema

A config file is a **top-level list** of resources (`models/src/iaac`), and the file is the
**source of truth** for everything it declares. Update routes take the whole object and rewrite
every field, so apply must send a complete object — anything the file omits would otherwise be
reverted. Concretely:

- The schema mirrors API optionality: required on the API ⇒ required in the file. Unknown keys
  and missing required fields are both hard parse errors (`deny_unknown_fields`), so a typo can
  never silently mean "leave it alone".
- **Machine type and volumes aren't in the schema.** On update, `apply` reads the deployment
  back with `GetDeploymentInfo` and carries both over verbatim. New fields that the file can't
  describe must do the same, or applying will wipe them.
- **The registry can't change after create** — `UpdateDeployment` has no `registry` field. Apply
  compares against the existing deployment and errors on mismatch.
- **Patr images are `registry.patr.cloud/{workspace}/{repository}`** — that's the path the
  registry serves (`/v2/{workspace_id}/{repo_name}`), but `container_registry_repository.name`
  is only the part *after* the workspace. `IaacDeploymentImage` parses the whole thing into
  `repository`, so apply strips the workspace segment before looking the repository up. A
  leading segment that isn't a workspace UUID stays part of the name — names can contain
  slashes.
- `--dry-run` resolves every reference (so unresolvable names still fail) but issues no
  create/update call.

## Verification

`cargo check -p cli`, `cargo clippy -p cli --no-deps` (per-package), `cargo bindings` after touching `models`.

**Tests: `just cli::test`.** The suite (`cli/tests/`) drives commands against a `wiremock` stub
API and asserts on the exact request bodies. `constants::API_BASE_URL` is a compile-time
constant, so the recipe builds the tests with `PATR_TEST_API_BASE_URL` set — plain
`cargo test -p cli` will point them at a real API and fail. One fixed port means one shared
server, hence `--test-threads=1`.
