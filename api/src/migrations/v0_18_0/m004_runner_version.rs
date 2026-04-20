//! Add a `version` column to the `runner` table recording the semver of the
//! last runner binary to connect. Existing rows are backfilled with `0.0.0`
//! so the UI flags them as outdated until they reconnect and report their
//! real version. The default is dropped so future inserts must supply a
//! value explicitly.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE runner
		ADD COLUMN version TEXT NOT NULL DEFAULT '0.0.0';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE runner
		ALTER COLUMN version DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
