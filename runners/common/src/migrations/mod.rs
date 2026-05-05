//! Database migrations organized by version.
//!
//! Each migration registers itself via `inventory::submit!`. The runner
//! collects all registered migrations, sorts by version then name, and
//! executes any that haven't been applied yet.
//!
//! **Important:** Migrations must use `sqlx::query()` (runtime), not the
//! `query!` macro (compile-time), because migrations alter the schema.

use std::{collections::HashSet, future::Future, pin::Pin};

use semver::Version;
use sqlx::Connection as _;

use crate::prelude::*;

// Import version modules so their inventory::submit! calls are linked
mod v0_18_0;

/// The function signature for a migration: takes a mutable DB connection
/// reference and returns a pinned, boxed future that resolves to a sqlx Result.
type MigrateFn = for<'a> fn(
	&'a mut DatabaseConnection,
) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>> + Send + 'a>>;

/// A registered database migration.
///
/// Each migration file submits one of these via `inventory::submit!`.
pub struct Migration {
	/// Migration name, e.g. `"m001_initial_baseline"`
	pub name: &'static str,
	/// The version this migration belongs to
	pub version: Version,
	/// The migration function
	pub migrate: MigrateFn,
}

inventory::collect!(Migration);

/// Runs pending migrations from `from_version` onwards (inclusive, to cover
/// alpha -> stable channel switches where the version matches but new
/// migrations may exist). Skips anything already recorded in the
/// `migrations` table.
pub async fn run_migrations(
	connection: &mut DatabaseConnection,
	from_version: &Version,
) -> Result<(), sqlx::Error> {
	let applied = query(
		r#"
		SELECT
			name
		FROM
			migrations;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<String, _>("name"))
	.collect::<HashSet<String>>();

	let mut migrations = inventory::iter::<Migration>
		.into_iter()
		.filter(|m| m.version >= *from_version)
		.collect::<Vec<_>>();
	migrations.sort_by(|a, b| a.version.cmp(&b.version).then(a.name.cmp(b.name)));

	for migration in migrations {
		if applied.contains(migration.name) {
			continue;
		}

		info!("Running migration: {}", migration.name);

		// Run the migration and record it in the same transaction so a partial
		// failure rolls back any DDL — otherwise leftover temp tables (e.g.
		// `*_new`) wedge subsequent retries on `CREATE TABLE`.
		let mut txn = connection.begin().await?;

		(migration.migrate)(&mut *txn).await?;

		let version = migration.version.to_string();
		query(
			r#"
			INSERT INTO
				migrations(name, version)
			VALUES
				($1, $2);
			"#,
		)
		.bind(migration.name)
		.bind(&version)
		.execute(&mut *txn)
		.await?;

		txn.commit().await?;

		info!("Migration applied: {}", migration.name);
	}

	Ok(())
}

/// Marks all registered migrations as applied without running them. Used
/// on fresh database initialization where the schema is already current.
pub async fn mark_all_applied(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	for migration in inventory::iter::<Migration> {
		let version = migration.version.to_string();
		query(
			r#"
			INSERT INTO
				migrations(name, version)
			VALUES
				($1, $2)
			ON CONFLICT
			DO NOTHING;
			"#,
		)
		.bind(migration.name)
		.bind(&version)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
