//! Adds `secret.last_updated`.
//!
//! - `initialize_secret_tables` now creates the column, so migrated databases need it too.
//! - Existing rows are backfilled from `resource.created` before `NOT NULL` goes on.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE secret
			ADD COLUMN last_updated TIMESTAMPTZ;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			secret
		SET
			last_updated = resource.created
		FROM
			resource
		WHERE
			resource.id = secret.id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE secret
			ALTER COLUMN last_updated SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
