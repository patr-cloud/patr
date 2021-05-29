use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	models::db_mapping::{DeploymentUpgradePath, MachineType},
	query,
	query_as,
};

pub async fn initialize_upgrade_path_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing upgrade path tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_machine_type (
			id BINARY(16) PRIMARY KEY,
			cpu_count TINYINT UNSIGNED NOT NULL,
			memory_count FLOAT UNSIGNED NOT NULL,
			UNIQUE(cpu_count, memory_count)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_upgrade_path(
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			default_machine_type BINARY(16)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_upgrade_path_machine_type (
			upgrade_path_id BINARY(16) NOT NULL,
			machine_type_id BINARY(16) NOT NULL,
			PRIMARY KEY (upgrade_path_id, machine_type_id),
			FOREIGN KEY(upgrade_path_id) REFERENCES deployment_upgrade_path(id),
			FOREIGN KEY(machine_type_id) REFERENCES deployment_machine_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_upgrade_path_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_upgrade_path
		ADD CONSTRAINT
		FOREIGN KEY (id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn generate_new_deployment_machine_type_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut exists = query!(
		r#"
		SELECT
			*
		FROM
			deployment_machine_type
		WHERE
			id = ?;
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
				id = ?;
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
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<DeploymentUpgradePath>, sqlx::Error> {
	query_as!(
		DeploymentUpgradePath,
		r#"
		SELECT
			deployment_upgrade_path.*
		FROM
			deployment_upgrade_path,
			resource
		WHERE
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_machine_types_in_deployment_upgrade_path(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
) -> Result<Vec<MachineType>, sqlx::Error> {
	query_as!(
		MachineType,
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
			deployment_upgrade_path_machine_type.upgrade_path_id = ?;
		"#,
		upgrade_path_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_deployment_upgrade_path_by_id(
	connection: &mut Transaction<'_, MySql>,
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
			id = ?;
		"#,
		upgrade_path_id
	)
	.fetch_all(connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn get_deployment_upgrade_path_by_name_in_organisation(
	connection: &mut Transaction<'_, MySql>,
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
			deployment_upgrade_path.name = ? AND
			resource.owner_id = ?;
		"#,
		name,
		organisation_id
	)
	.fetch_all(connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn create_deployment_upgrade_path(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_upgrade_path
		VALUES
			(?, ?, NULL);
		"#,
		upgrade_path_id,
		name
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_machine_type_id_for_configuration(
	connection: &mut Transaction<'_, MySql>,
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
			cpu_count = ? AND
			memory_count = ?;
		"#,
		cpu_count,
		memory_count
	)
	.fetch_all(connection)
	.await
	.map(|rows| rows.into_iter().next().map(|row| row.id))
}

pub async fn add_deployment_machine_type(
	connection: &mut Transaction<'_, MySql>,
	machine_type_id: &[u8],
	cpu_count: u8,
	memory_count: f32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_machine_type
		VALUES
			(?, ?, ?);
		"#,
		machine_type_id,
		cpu_count,
		memory_count
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_machine_type_for_upgrade_path(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
	machine_type_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_upgrade_path_machine_type
		VALUES
			(?, ?);
		"#,
		upgrade_path_id,
		machine_type_id,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_machine_types_for_deployment_upgrade_path(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_upgrade_path_machine_type
		WHERE
			upgrade_path_id = ?;
		"#,
		upgrade_path_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_upgrade_path_name_by_id(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_upgrade_path
		SET
			name = ?
		WHERE
			id = ?;
		"#,
		name,
		upgrade_path_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn delete_deployment_upgrade_path_by_id(
	connection: &mut Transaction<'_, MySql>,
	upgrade_path_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_upgrade_path
		WHERE
			id = ?;
		"#,
		upgrade_path_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
