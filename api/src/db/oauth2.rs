use sqlx::{MySql, Transaction};

use crate::{query, query_as};

pub async fn initialize_oauth_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing Portus tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS oauth_client(
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			secret_key BINARY(64) NOT NULL,
			redirect_url VARCHAR(100) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}

// query to enter data into db
pub async fn oauth_register_client(
	connection: &mut Transaction<'_, MySql>,
	id: &[u8],
	name: &str,
	redirect_url: &str,
	secret_key: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			oauth_client
		VALUES
			(?,?,?,?);
		"#,
		id,
		name,
		secret_key,
		redirect_url
	)
	.execute(connection)
	.await?;

	Ok(())
}

// query to check if redirect url exists in the database
// query to check if client exists in the database
