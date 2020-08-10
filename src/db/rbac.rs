use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_rbac(
	mut transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS resources (
			resourceId BINARY(16) PRIMARY KEY
		);
		"#
	)
	.execute(&mut transaction)
	.await?;
	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS roles (
			resourceId BINARY(16) PRIMARY KEY
		);
		"#
	)
	.execute(&mut transaction)
	.await?;
	Ok(())
}
