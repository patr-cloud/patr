---
name: migrations
description: Write and verify a Postgres schema migration for the api. Use whenever a change alters the database schema (tables, columns, constraints, indexes, types, functions) or data that existing workspaces need, such as permissions granted to default roles. Verifies the migration against a clone of alpha's schema, because tests never run migration bodies.
---

# Migrations

Fresh databases never run migrations. That includes every test, every CI job and every local run. `api/src/db/initializer.rs` builds the schema from the `initialize_*` DDL in `api/src/db/`, then marks every migration as applied without executing it. So a migration can be broken, or produce a different schema from the fresh-install DDL, and everything still goes green. The only real test is running it against a copy of a real database.

## Writing it

1. Scaffold with `cargo new-migration <name>`. That creates `api/src/migrations/vX_Y_Z/mNNN_<name>.rs`; register it in that version's `mod.rs`. Number it after the latest migration on `develop`. If `develop` gains a migration before yours merges, renumber yours to come after it.
2. Never change a migration that has already been deployed. Fix forward with a new one.
3. Use runtime `sqlx::query`, never `query!`, since the schema is mid-change. Each migration runs in a transaction.
4. Change the fresh-install DDL in `api/src/db/` in the same change, so that both paths produce the same schema. A function that a migration redefines must match the DDL's copy once whitespace is normalised.
5. Make it safe on the data that's actually there:
   - backfill before adding `NOT NULL`;
   - check existing rows satisfy any new constraint;
   - guard renames and drops whose target might differ between databases.
6. If existing workspaces need new data, such as permissions for their default roles, the migration grants it. `default_roles()` in `api/src/routes/api.patr.cloud/workspace/create_workspace.rs` covers new workspaces only.
7. Run `just prepare` if any `query!` changed.

Watch for these:
- **Backslashes in raw strings.** A Rust raw string (`r#"…"#`) passes them through literally. A regex `\.` is written `\.`; writing `\\.` puts two backslashes into the SQL.
- **Postgres 18 names `NOT NULL` constraints**, as `<table>_<column>_not_null`. Renaming a column doesn't rename its constraint, so the old name lingers unless you rename that too.
- **`ADD COLUMN` appends.** If the fresh-install DDL puts the column somewhere else, the two schemas differ in column order.

## Verifying against alpha

**Alpha's database is read-only to you.** Dump from it and run `SELECT`s against it. Never write to it, and never run a migration against it. Alpha's address is in the maintainer's local instructions.

Work in your scratchpad directory, not the repo.

1. **Build** with `cargo build -p api`.
2. **Dump alpha's schema and its migrations table:**
   ```sh
   ssh <alpha> 'cd /root/patr && docker compose exec -T postgres pg_dump -U postgres -d api --schema-only --no-owner --no-privileges' > alpha-schema.sql
   ssh <alpha> 'cd /root/patr && docker compose exec -T postgres pg_dump -U postgres -d api --data-only --no-owner -t migrations' > alpha-migrations.sql
   ```
3. **Start a throwaway Postgres** on the same image as alpha (`postgis/postgis:18-3.6`), and a Redis, both on spare ports. Wait for "PostgreSQL init process complete" in the logs before connecting. Create two databases, `fresh` and `alpha`, and restore both dumps into `alpha`.
4. **Migrate both databases with the built binary:**
   ```sh
   PATR__DATABASE__{HOST,PORT,USER,PASSWORD,DATABASE}=… PATR__REDIS__{HOST,PORT}=… <repo>/target/debug/api --migrate
   ```
   - Run it from a scratch directory holding a copy of `config/api.json`, with every required block filled in. A camelCase key is set by splitting it into snake case in the env var name: `openBao` is `PATR__OPEN_BAO__…`. `PATR__OPENBAO__…` silently doesn't match.
   - On `alpha`, the log must show your migration running, along with any pending migrations before it.
   - A panic about a schema that already exists, coming from the job queue after the migrations have finished, is an artefact of the clone, not a failure.
5. **Dump both schemas, normalise whitespace, and diff them.**
   - Expected differences: extensions that only alpha has (PostGIS, `pg_stat_statements`) and whitespace inside function bodies.
   - Any other difference is a bug, in the migration or in the fresh-install DDL. Fix it or explain it.
6. **Check the data:**
   - Seed representative rows before migrating the clone, and confirm they survive.
   - For a new or changed constraint, confirm with a read-only `SELECT` on alpha that alpha's existing rows satisfy it.
   - For RBAC changes, compare effective permissions before and after by calling `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(login_id, permission_name)` for every login and permission, and explain every difference.
7. **Check the guards,** if the migration is meant to be safe on any schema. Delete its row from `migrations` in `fresh` and migrate again. It should succeed and change nothing.
8. **Clean up** the containers and the dumps.

## Report

Say:
- what the migration does;
- the diff result, listing the expected differences you ignored;
- what the data checks showed;
- anything still unresolved.
