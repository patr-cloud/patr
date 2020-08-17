use crate::query;

use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_organisations(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing organisation tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			super_admin_id BINARY(16) NOT NULL,
			active BOOL NOT NULL DEFAULT FALSE,
			FOREIGN KEY(super_admin_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}
