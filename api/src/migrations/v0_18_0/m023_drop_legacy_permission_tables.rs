//! Drops the legacy per-permission include/exclude tables now that nothing
//! reads or writes them: role scopes live on `role_binding`, token ceilings
//! on `user_api_token_role_binding` (both backfilled in m017 and cut over in
//! m018 / m019). The `PERMISSION_TYPE` enum goes with them.
//!
//! `user_api_token_workspace_permission_type` and its `TOKEN_PERMISSION_TYPE`
//! enum stay — the super-admin arm of a token's ceiling still lives there.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DROP TABLE role_resource_permissions_include;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE role_resource_permissions_exclude;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE role_resource_permissions_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE user_api_token_resource_permissions_include;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE user_api_token_resource_permissions_exclude;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE user_api_token_resource_permissions_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query("DROP TYPE PERMISSION_TYPE;")
		.execute(&mut *connection)
		.await?;

	Ok(())
}
