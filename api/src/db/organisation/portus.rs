use sqlx::{MySql, Transaction};

use crate::{models::db_mapping::PortusTunnel, query, query_as};

pub async fn initialize_portus_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing Portus tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS portus_tunnels (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) NOT NULL,
			ssh_port SMALLINT UNSIGNED NOT NULL,
			exposed_port SMALLINT UNSIGNED NOT NULL,
			tunnel_name VARCHAR(50) NOT NULL,
			created BIGINT UNSIGNED NOT NULL
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
		ALTER TABLE portus_tunnels
		ADD CONSTRAINT 
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
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
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			portus_tunnels
		VALUES
			(?, ?, ?, ?, ?, ?);
		"#,
		id,
		username,
		ssh_port,
		exposed_port,
		created,
		tunnel_name,
	)
	.execute(connection)
	.await?;
	Ok(())
}

/// function to check if container exists with the given tunnel name
pub async fn get_portus_tunnel_by_name(
	connection: &mut Transaction<'_, MySql>,
	tunnel_name: &str,
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let rows = query_as!(
		PortusTunnel,
		r#"
		SELECT
			*
		FROM
			portus_tunnels
		WHERE
			tunnel_name = ?;
		"#,
		tunnel_name
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_portus_tunnel_by_tunnel_id(
	connection: &mut Transaction<'_, MySql>,
	tunnel_id: &[u8],
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let rows = query_as!(
		PortusTunnel,
		r#"
		SELECT
			*
		FROM
			portus_tunnels
		WHERE
			id = ?;
		"#,
		tunnel_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn is_portus_port_available(
	connection: &mut Transaction<'_, MySql>,
	port: u32,
) -> Result<bool, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnels
		WHERE
			ssh_port = ? OR
			exposed_port = ?;
		"#,
		port,
		port
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.is_empty())
}

pub async fn get_portus_tunnels_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<PortusTunnel>, sqlx::Error> {
	query_as!(
		PortusTunnel,
		r#"
		SELECT 
			portus_tunnels.*
		FROM 
			portus_tunnels
		INNER JOIN 
			resource 
		ON 
			resource.id = portus_tunnels.id
		WHERE
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}
