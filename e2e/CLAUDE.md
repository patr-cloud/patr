# e2e/CLAUDE.md

Playwright end-to-end suite. A **standalone pnpm project**, not part of the frontend workspace. See root `CLAUDE.md` for workspace-wide rules.

## Running

Lifecycle is driven by `just` (from `e2e/`): `just` = `install → up → test → down`.

- `just install` — pnpm install + `playwright install chromium`, **and builds the production frontend**. Tests run against the built frontend, served via Caddy on `:3001`.
- `just up` — `docker compose up -d --wait` (services on **offset ports**: postgres `:15432`, redis `:16379`, minio `:19000`, loki `:13100`, mimir `:18080` — nothing on default ports, to avoid clashing with a dev stack).
- `just test [args]` — starts node mocks + API (`:3000`) + frontend (`:13030`), then runs Playwright in two passes (`@racy` tests run last, serial).
- `just serve` / `just stop` — bring the stack up with the frontend in **dev** mode for interactive work.
- `DOCKER_VERSIONS=27 just up test`, `TEST_THREADS=4 just test`, `API_BIN=/path just test` — version matrix / parallelism / reuse a prebuilt API.
- `DASHBOARD_URL=… API_DIRECT_URL=… playwright test` — point the suite at a stack on non-default ports (e.g. when something else holds `:3000`); the corresponding `PATR__SERVER__BIND_ADDRESS` must match.

Running `playwright test` on its own boots nothing — use `just test` (or `just serve` first). Config injected via `PATR__SECTION__KEY` env vars in the `Justfile`; `helpers/config.ts` mirrors some and must stay in sync.

## Conventions

- **Prettier: tabs**, tabWidth 4, printWidth 100, **single quotes**, `trailingComma "all"`, semicolons. Format only — no eslint here. `pnpm format` / `pnpm format:check`.
- Tests: `test.describe('feature > action [UI]', ...)`; UI flows go through the page-object helpers in `helpers/ui/*`; users are `AsyncDisposable` (`await using user = await createUserAccount(api)`).
- **URL matters**: `DASHBOARD_URL` (`:3001`, cookie auth, `/api/**` proxy) vs `API_DIRECT_URL` (`:3000`, Bearer). **API tokens must use `API_DIRECT_URL`** — the proxy 400s on Bearer.
- Two auth paths: programmatic `loginAs(context, user, {...})` (fast, default) vs real UI login (only in auth specs).

## Gotchas

- **Tests run in cloud mode** (build API with default features) **for now** — they'll switch to self-hosted once that feature is fleshed out.
- **Serial by default** (`workers: 1`) — the shared dev stack (single Postgres, one API binary, Vinxi HMR) breaks under concurrency. Opt into parallelism with `TEST_THREADS`. Flaky concurrency/navigation specs are tagged `@racy` and run serially in a second pass.
- **Vinxi HMR breaks naive Playwright waits**: use `waitUntil: 'domcontentloaded'` (default `'load'` never fires on `_workspaced` routes), use the custom `expectUrl` helpers (not `toHaveURL` polling), and **never `page.reload()`** (it hangs — open a new tab in the same context instead). Narrow route interception to `/api/**`, not `/**`, or HMR requests starve the scheduler.
- **Debug-build shortcuts**: OTP is always `000000`, Turnstile token `1x00000000000000000000AA` always passes. **Don't skip email flows** — use the debug OTP, or read the secret straight from Postgres/Redis via `helpers/db.ts` / `helpers/redis.ts`.
- **Cookie origin**: `loginAs` must set cookies on `:3001` (the Caddy origin the SPA loads from), not `:13030`, or they aren't sent → the `_logged-in` guard bounces to `/login`.
- `@docker` suite = real Docker-in-Docker, matrix over Docker 26–29 (`--project=docker-<v>`). Needs `cargo build -p docker` first, or the harness gets ENOENT.
- **Don't judge flakiness from the last run** — baseline against an average of 4–5 green runs.
