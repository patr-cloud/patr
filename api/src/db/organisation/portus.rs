use sqlx::{MySql, Transaction};

use crate::query;

pub async fn initialize_portus_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing Portus tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS portus (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100),
			sshPort INTEGER,
			exposedPort INTEGER,
			tunnelName VARCHAR(50)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}

pub async fn initialize_portus_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE portus
		ADD CONSTRAINT 
		FOREIGN KEY(id) REFERENCES resource(id);"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

// query to add user information with port and container details
pub async fn add_user_for_portus(
	connection: &mut Transaction<'_, MySql>,
	id: &[u8],
	username: &str,
	ssh_port: u32,
	exposed_port: u32,
	tunnel_name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			portus
		VALUES
			(?,?,?,?,?);
		"#,
		id,
		username,
		ssh_port,
		exposed_port,
		tunnel_name,
	)
	.execute(connection)
	.await?;
	Ok(())
}

/// function to check if container exists with the given tunnel name
pub async fn check_if_tunnel_exists(
	connection: &mut Transaction<'_, MySql>,
	tunnel_name: &str,
) -> Result<bool, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			portus
		WHERE
			tunnelName = ?;
		"#,
		tunnel_name
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(false);
	}

	return Ok(true);
}
