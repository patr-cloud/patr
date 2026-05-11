//! Drops the `deployment_volume` and `deployment_volume_mount` tables, plus
//! the matching RBAC seed rows. The volume feature is being removed entirely.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS deployment_volume_mount;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE IF EXISTS deployment_volume;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	for table in [
		"role_resource_permissions_include",
		"role_resource_permissions_exclude",
		"role_resource_permissions_type",
		"user_api_token_resource_permissions_include",
		"user_api_token_resource_permissions_exclude",
		"user_api_token_resource_permissions_type",
	] {
		sqlx::query(&format!(
			r#"
			DELETE FROM {table}
			WHERE permission_id IN (
				SELECT id FROM permission
				WHERE name LIKE 'volume::%'
			);
			"#,
		))
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(
		r#"
		DELETE FROM permission
		WHERE name LIKE 'volume::%';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM resource_type
		WHERE name = 'volume';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
