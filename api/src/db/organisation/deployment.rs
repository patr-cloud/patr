use sqlx::{MySql, Transaction};

use crate::{
	models::db_mapping::{
		Deployment,
		DockerRepository,
		EnvVariable,
		MachineType,
		VolumeMount,
	},
	query,
	query_as,
};
pub async fn initialize_deployer_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT "registry.docker.vicara.co",
			repositoryId BINARY(16),
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			domain_id BINARY(16) NOT NULL,
			sub_domain VARCHAR(255) NOT NULL,
			path VARCHAR(255) NOT NULL DEFAULT "/",
			/* TODO change port to port array, and take image from docker_registry_repository */
			persistence BOOL NOT NULL,
			datacenter VARCHAR(255) NOT NULL,
			UNIQUE (domain_id, sub_domain, path),
			CONSTRAINT CHECK (
				(
					registry = "docker.registry.vicara.co" AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				) OR
				(
					registry != "docker.registry.vicara.co" AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS docker_registry_repository (
			id BINARY(16) PRIMARY KEY,
			organisation_id BINARY(16) NOT NULL,
			name VARCHAR(255) NOT NULL,
			UNIQUE(organisation_id, name)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	// change this table to deployment id and port as unique
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS port (
			deployment_id BINARY(16),
			port SMALLINT UNSIGNED NOT NULL,
			PRIMARY KEY (deployment_id, port),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS environment_variable (
			deployment_id BINARY(16),
			name VARCHAR(50) NOT NULL,
			value VARCHAR(50) NOT NULL,
			PRIMARY KEY (deployment_id, name),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS volume(
			deployment_id BINARY(16),
			name VARCHAR(50) NOT NULL,
			path VARCHAR(255) NOT NULL,
			PRIMARY KEY (deployment_id, name),
			UNIQUE (deployment_id, path),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_gpu_type (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) NOT NULL UNIQUE
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS default_deployment_machine_type (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL UNIQUE,
			cpu_count TINYINT UNSIGNED NOT NULL,
			memory_count FLOAT UNSIGNED NOT NULL,
			gpu_type_id BINARY(16) NOT NULL,
			UNIQUE(cpu_count, memory_count, gpu_type_id),
			FOREIGN KEY(gpu_type_id) REFERENCES deployment_gpu_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_machine_type (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL UNIQUE,
			cpu_count TINYINT UNSIGNED NOT NULL,
			memory_count FLOAT UNSIGNED NOT NULL,
			gpu_type_id BINARY(16) NOT NULL,
			UNIQUE(cpu_count, memory_count, gpu_type_id),
			FOREIGN KEY(gpu_type_id) REFERENCES deployment_gpu_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	// CREATE TABLE IF NOT EXISTS deployment_machine_type ( id BINARY(16)
	// PRIMARY KEY, name VARCHAR(100) NOT NULL UNIQUE, cpu_count SMALLINT
	// UNSIGNED NOT NULL, memory_count FLOAT UNSIGNED NOT NULL,yer_gpu_type(id)
	// );
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_upgrade_path (
			deployment_id BINARY(16) UNIQUE NOT NULL,
			machine_type_id BINARY(16) NOT NULL,
			PRIMARY KEY (deployment_id, machine_type_id),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			FOREIGN KEY(machine_type_id) REFERENCES deployment_machine_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}
pub async fn initialize_deployer_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT
		FOREIGN KEY (id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT
		FOREIGN KEY (repositoryId) REFERENCES docker_registry_repository(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT
		FOREIGN KEY (id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}
// function to add new repositorys
pub async fn create_repository(
	transaction: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
	name: &str,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			docker_registry_repository
		VALUES
			(?, ?, ?);
		"#,
		resource_id,
		organisation_id,
		name
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}
pub async fn get_repository_by_name(
	connection: &mut Transaction<'_, MySql>,
	repository_name: &str,
	organisation_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	let rows = query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			name = ?
		AND
			organisation_id = ?;
		"#,
		repository_name,
		organisation_id
	)
	.fetch_all(connection)
	.await?;
	Ok(rows.into_iter().next())
}
pub async fn get_docker_repositories_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			organisation_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}
pub async fn get_docker_repository_by_id(
	connection: &mut Transaction<'_, MySql>,
	repository_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			id = ?;
		"#,
		repository_id
	)
	.fetch_all(connection)
	.await
	.map(|repos| repos.into_iter().next())
}
pub async fn delete_docker_repository_by_id(
	connection: &mut Transaction<'_, MySql>,
	repository_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository
		WHERE
			id = ?;
		"#,
		repository_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
pub async fn create_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	name: &str,
	registry: &str,
	repository_id: Option<Vec<u8>>,
	image_name: &str,
	image_tag: &str,
	domain_id: &[u8],
	sub_domain: &str,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?, ?, false, 'india');
		"#,
		deployment_id,
		name,
		registry,
		repository_id.unwrap(),
		image_name,
		image_tag,
		domain_id,
		sub_domain,
		path
	)
	.execute(connection)
	.await
	.map(|_| ())
}
pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	image_name: &str,
	image_tag: &str,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id,
			deployment.name,
			deployment.registry,
			deployment.image_name,
			deployment.image_tag,
			deployment.domain_id,
			deployment.sub_domain,
			deployment.path
		FROM
			deployment,
            resource
		WHERE
            deployment.id = resource.id AND
			image_name = ? AND
			image_tag = ? AND
            resource.owner_id = ?;
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(connection)
	.await
}
pub async fn get_deployments_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id,
			deployment.name,
			deployment.registry,
			deployment.image_name,
			deployment.image_tag,
			deployment.domain_id,
			deployment.sub_domain,
			deployment.path
		FROM
			deployment,
			resource
		WHERE
			resource.id = deployment.id AND
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}
pub async fn get_deployment_by_id(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	Ok(query_as!(
		Deployment,
		r#"
			SELECT
				id,
				name,
				registry,
				image_name,
				image_tag,
				domain_id,
				sub_domain,
				path
			FROM
				deployment
			WHERE
				id = ?;
			"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.next())
}
pub async fn get_deployment_by_entry_point(
	connection: &mut Transaction<'_, MySql>,
	domain_id: &[u8],
	sub_domain: &str,
	path: &str,
) -> Result<Option<Deployment>, sqlx::Error> {
	Ok(query_as!(
		Deployment,
		r#"
			SELECT
				id,
				name,
				registry,
				image_name,
				image_tag,
				domain_id,
				sub_domain,
				path
			FROM
				deployment
			WHERE
				domain_id = ? AND
				sub_domain = ? AND
				path = ?;
			"#,
		domain_id,
		sub_domain,
		path
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.next())
}
pub async fn delete_deployment_by_id(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = ?;
		"#,
		deployment_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
pub async fn create_deployment_machine_type(
	connection: &mut Transaction<'_, MySql>,
	machine_type_id: &[u8],
	name: &str,
	cpu_count: u8,
	memory_count: f32,
	gpu_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
	INSERT INTO 
		deployment_machine_type
	VALUES
		(?, ?, ?, ?, ?);
	"#,
		machine_type_id,
		name,
		cpu_count,
		memory_count,
		gpu_id,
	)
	.execute(connection)
	.await
	.map(|_| ())
}
//  here we will not need id as the aim is to see
// if a mcchine type with the given config already exists
pub async fn get_deployment_machine_type(
	connection: &mut Transaction<'_, MySql>,
	name: &str,
	cpu_count: u8,
	memory_count: f32,
	gpu_id: &[u8],
) -> Result<Option<MachineType>, sqlx::Error> {
	Ok(query_as!(
		MachineType,
		r#"
			SELECT
				*
			FROM
				deployment_machine_type
			WHERE
				name = ? AND
				cpu_count = ? AND
				memory_count = ? AND
				gpu_type_id = ?;
			"#,
		name,
		cpu_count,
		memory_count,
		gpu_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.next())
}
pub async fn insert_deployment_port(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
	INSERT INTO 
		port
	VALUES
		(?, ?);
	"#,
		deployment_id,
		port
	)
	.execute(connection)
	.await
	.map(|_| ())
}
pub async fn insert_deployment_volumes(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	name: &str,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
	INSERT INTO 
		volume
	VALUES
		(?, ?, ?);
	"#,
		deployment_id,
		name,
		path
	)
	.execute(connection)
	.await
	.map(|_| ())
}
pub async fn insert_deployment_environment_variables(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	name: &str,
	value: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
	INSERT INTO 
		environment_variable
	VALUES
		(?, ?, ?);
	"#,
		deployment_id,
		name,
		value
	)
	.execute(connection)
	.await
	.map(|_| ())
}
// function to return list of ports
pub async fn get_ports_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<u8>, sqlx::Error> {
	Ok(query!(
		r#"
			SELECT
				port
			FROM
				port
			WHERE
				deployment_id = ?;
			"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|val| val.port as u8)
	.collect())
}
// function to return list of env variables
pub async fn get_variables_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<EnvVariable>, sqlx::Error> {
	query_as!(
		EnvVariable,
		r#"
			SELECT
				*
			FROM
				environment_variable
			WHERE
				deployment_id = ?;
			"#,
		deployment_id
	)
	.fetch_all(connection)
	.await
}
// function to return list of volume mounts
pub async fn get_volume_mounts_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<VolumeMount>, sqlx::Error> {
	query_as!(
		VolumeMount,
		r#"
			SELECT
				*
			FROM
				volume
			WHERE
				deployment_id = ?;
			"#,
		deployment_id
	)
	.fetch_all(connection)
	.await
}
