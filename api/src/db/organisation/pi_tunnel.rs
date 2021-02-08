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
            containerId VARCHAR(50)
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
	container_id: &str,
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
		container_id,
	)
	.execute(connection)
	.await?;
	Ok(())
}
