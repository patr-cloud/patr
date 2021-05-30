use sqlx::Transaction;
use uuid::Uuid;

use crate::{
	models::db_mapping::{DeploymentUpgradePath, MachineType},
	query,
	query_as,
	Database,
};

pub async fn initialize_upgrade_path_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing upgrade path tables");
	query!(
		r#"
		CREATE TABLE deployment_machine_type(
			id BYTEA CONSTRAINT deployment_machine_type_pk PRIMARY KEY,
			cpu_count SMALLINT NOT NULL
				CONSTRAINT deployment_machine_type_chk_cpu_u8
					CHECK(cpu_count >= 0 AND cpu_count <= 256),
			memory_count REAL NOT NULL,
			CONSTRAINT deployment_machine_type_uq_config
				UNIQUE(cpu_count, memory_count)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_upgrade_path(
			id BYTEA CONSTRAINT deployment_upgrade_path_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			default_machine_type BYTEA NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_upgrade_path_machine_type(
			upgrade_path_id BYTEA NOT NULL
				CONSTRAINT
					deployment_upgrade_path_machine_type_fk_upgrade_path_id
					REFERENCES deployment_upgrade_path(id),
			machine_type_id BYTEA NOT NULL
				CONSTRAINT
					deployment_upgrade_path_machine_type_fk_machine_type_id
					REFERENCES deployment_machine_type(id),
			CONSTRAINT deployment_upgrade_path_machine_type_pk
				PRIMARY KEY (upgrade_path_id, machine_type_id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_upgrade_path
		ADD CONSTRAINT deployment_upgrade_path_fk_id_default_machine_type
		FOREIGN KEY(id, default_machine_type)
		REFERENCES deployment_upgrade_path_machine_type(
			upgrade_path_id,
			machine_type_id
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_upgrade_path_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up upgrade path tables initialization");
	query!(
		r#"
		ALTER TABLE deployment_upgrade_path
		ADD CONSTRAINT deployment_upgrade_path_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn generate_new_deployment_machine_type_id(
	connection: &mut Transaction<'_, Database>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut exists = query!(
		r#"
		SELECT
			*
		FROM
			deployment_machine_type
		WHERE
			id = $1;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.is_some();

	while exists {
		uuid = Uuid::new_v4();
		exists = query!(
			r#"
			SELECT
				*
			FROM
				deployment_machine_type
			WHERE
				id = $1;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.next()
		.is_some();
	}

	Ok(uuid)
}

pub async fn get_deployment_upgrade_paths_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<DeploymentUpgradePath>, sqlx::Error> {
	query_as!(
		DeploymentUpgradePath,
		r#"
		SELECT
			deployment_upgrade_path.*
		FROM
			deployment_upgrade_path
		INNER JOIN
			resource
		ON
			resource.id = deployment_upgrade_path.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_machine_types_in_deployment_upgrade_path(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
) -> Result<Vec<MachineType>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			deployment_machine_type.cpu_count,
			deployment_machine_type.memory_count
		FROM
			deployment_upgrade_path_machine_type
		INNER JOIN
			deployment_machine_type
		ON
			deployment_machine_type.id = deployment_upgrade_path_machine_type.machine_type_id
		WHERE
			deployment_upgrade_path_machine_type.upgrade_path_id = $1;
		"#,
		upgrade_path_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| MachineType {
		cpu_count: row.cpu_count as u8,
		memory_count: row.memory_count
	})
	.collect();

	Ok(rows)
}

pub async fn get_deployment_upgrade_path_by_id(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
) -> Result<Option<DeploymentUpgradePath>, sqlx::Error> {
	query_as!(
		DeploymentUpgradePath,
		r#"
		SELECT
			*
		FROM
			deployment_upgrade_path
		WHERE
			id = $1;
		"#,
		upgrade_path_id
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn get_deployment_upgrade_path_by_name_in_organisation(
	connection: &mut Transaction<'_, Database>,
	name: &str,
	organisation_id: &[u8],
) -> Result<Option<DeploymentUpgradePath>, sqlx::Error> {
	query_as!(
		DeploymentUpgradePath,
		r#"
		SELECT
			deployment_upgrade_path.*
		FROM
			deployment_upgrade_path
		INNER JOIN
			resource
		ON
			deployment_upgrade_path.id = resource.id
		WHERE
			deployment_upgrade_path.name = $1 AND
			resource.owner_id = $2;
		"#,
		name,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn create_deployment_upgrade_path(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_upgrade_path
		VALUES
			($1, $2, NULL);
		"#,
		upgrade_path_id,
		name
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_machine_type_id_for_configuration(
	connection: &mut Transaction<'_, Database>,
	cpu_count: u8,
	memory_count: f32,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
	query!(
		r#"
		SELECT
			*
		FROM
			deployment_machine_type
		WHERE
			cpu_count = $1 AND
			memory_count = $2;
		"#,
		cpu_count as i16,
		memory_count
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| rows.into_iter().next().map(|row| row.id))
}

pub async fn add_deployment_machine_type(
	connection: &mut Transaction<'_, Database>,
	machine_type_id: &[u8],
	cpu_count: u8,
	memory_count: f32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_machine_type
		VALUES
			($1, $2, $3);
		"#,
		machine_type_id,
		cpu_count as i16,
		memory_count
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_machine_type_for_upgrade_path(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
	machine_type_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_upgrade_path_machine_type
		VALUES
			($1, $2);
		"#,
		upgrade_path_id,
		machine_type_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_machine_types_for_deployment_upgrade_path(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_upgrade_path_machine_type
		WHERE
			upgrade_path_id = $1;
		"#,
		upgrade_path_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_upgrade_path_name_by_id(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_upgrade_path
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name,
		upgrade_path_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_deployment_upgrade_path_by_id(
	connection: &mut Transaction<'_, Database>,
	upgrade_path_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_upgrade_path
		WHERE
			id = $1;
		"#,
		upgrade_path_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
