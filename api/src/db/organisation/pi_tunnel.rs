use sqlx::{MySql, Transaction};

use crate::query;

pub async fn initialize_pi_tunnel_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing pi tunnel tables");
	query!(
		r#"
        CREATE TABLE IF NOT EXISTS pi_tunnel (
            id BINARY(16) PRIMARY KEY,
            username VARCHAR(100), 
            sshPort INTEGER, 
            exposedPort INTEGER, 
            containerName VARCHAR(50)
        );
        "#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}

pub async fn initialize_pi_tunnel_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
        ALTER TABLE pi_tunnel
        ADD CONSTRAINT 
        FOREIGN KEY(id) REFERENCES resource(id);"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

// query to add user information with port and container details
pub async fn add_user_for_pi_tunnel(
	connection: &mut Transaction<'_, MySql>,
	id: &[u8],
	username: &str,
	ssh_port: u32,
	exposed_port: u32,
	container_name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
        INSERT INTO
            pi_tunnel
        VALUES
            (?,?,?,?,?);
        "#,
		id,
		username,
		ssh_port,
		exposed_port,
		container_name,
	)
	.execute(connection)
	.await?;
	Ok(())
}

/// function to check if container exists
pub async fn check_if_container_exists(
	connection: &mut Transaction<'_, MySql>,
	container_name: &str,
) -> Result<Option<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT 
			*
		FROM
			pi_tunnel
		WHERE
			containerName = ?;
		"#,
		container_name
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	let row = rows.into_iter().next().unwrap();
	Ok(Some(row.containerName.unwrap()))
}
