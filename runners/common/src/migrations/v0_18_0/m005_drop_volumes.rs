//! Drops the `deployment_volume` and `deployment_volume_mount` tables.
//!
//! The volume feature is being removed entirely.

use crate::prelude::*;

/// Drop the volume-related tables.
#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	query("DROP TABLE IF EXISTS deployment_volume_mount;")
		.execute(&mut *connection)
		.await?;

	query("DROP TABLE IF EXISTS deployment_volume;")
		.execute(&mut *connection)
		.await?;

	Ok(())
}
