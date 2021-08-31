use crate::{models::db_mapping::PortusTunnel, query, Database};

pub async fn initialize_portus_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing portus tables");
	query!(
		r#"
		CREATE TABLE portus_tunnel(
			id BYTEA CONSTRAINT portus_tunnel_pk PRIMARY KEY,
			username VARCHAR(100) NOT NULL,
			ssh_port INTEGER NOT NULL
				CONSTRAINT portus_tunnel_chk_ssh_port_u16
					CHECK(ssh_port >= 0 AND ssh_port <= 65534),
			exposed_port INTEGER NOT NULL
				CONSTRAINT portus_tunnel_chk_exposed_port_u16
					CHECK(exposed_port >= 0 AND exposed_port <= 65534),
			name VARCHAR(50) NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			portus_tunnel_idx_name
		ON
			portus_tunnel
		(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_portus_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up portus tables initialization");
	query!(
		r#"
		ALTER TABLE portus_tunnel
		ADD CONSTRAINT portus_tunnel_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

// query to add user information with port and container details
pub async fn create_new_portus_tunnel(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	username: &str,
	ssh_port: u16,
	exposed_port: u16,
	tunnel_name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			portus_tunnel
		VALUES
			($1, $2, $3, $4, $5);
		"#,
		id,
		username,
		ssh_port as i32,
		exposed_port as i32,
		tunnel_name
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

// query to remove portus tunnel from database
pub async fn delete_portus_tunnel(
	connection: &mut <Database as sqlx::Database>::Connection,
	tunnel_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			portus_tunnel
		WHERE
			id = $1;
		"#,
		tunnel_id
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

/// function to check if container exists with the given tunnel name
pub async fn get_portus_tunnel_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	tunnel_name: &str,
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			name = $1;
		"#,
		tunnel_name
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
	});

	Ok(rows.next())
}

pub async fn get_portus_tunnel_by_tunnel_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	tunnel_id: &[u8],
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			id = $1;
		"#,
		tunnel_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
	});

	Ok(rows.next())
}

pub async fn is_portus_port_available(
	connection: &mut <Database as sqlx::Database>::Connection,
	port: u16,
) -> Result<bool, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			ssh_port = $1 OR
			exposed_port = $1;
		"#,
		port as i32
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
	});

	Ok(rows.next().is_none())
}

pub async fn get_portus_tunnels_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<PortusTunnel>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT 
			portus_tunnel.*
		FROM 
			portus_tunnel
		INNER JOIN 
			resource 
		ON 
			resource.id = portus_tunnel.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
	})
	.collect();

	Ok(rows)
}
