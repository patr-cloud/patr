//! Add the `secret` table, which tracks when each secret's value last changed.
//!
//! Deployments compare it against the versions they last applied, so a
//! rotated secret re-applies the deployments that use it.

use crate::prelude::*;

/// Create the `secret` table.
#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	query(
		r#"
		CREATE TABLE secret(
			id TEXT NOT NULL PRIMARY KEY,
			last_updated DATETIME NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
