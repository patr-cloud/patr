use api_models::{
	models::workspace::infrastructure::deployment::{
		DeploymentProbe,
		DeploymentStatus,
		ExposedPortType,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::{
	db::{self, WorkspaceAuditLog},
	models::deployment::DEFAULT_MACHINE_TYPES,
	query,
	query_as,
	Database,
};

pub struct DeploymentMachineType {
	pub id: Uuid,
	pub cpu_count: i16,
	pub memory_count: i32,
}

pub struct Deployment {
	pub id: Uuid,
	pub name: String,
	pub registry: String,
	pub repository_id: Option<Uuid>,
	pub image_name: Option<String>,
	pub image_tag: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
	pub region: Uuid,
	pub min_horizontal_scale: i16,
	pub max_horizontal_scale: i16,
	pub machine_type: Uuid,
	pub deploy_on_push: bool,
	pub startup_probe_port: Option<i32>,
	pub startup_probe_path: Option<String>,
	pub liveness_probe_port: Option<i32>,
	pub liveness_probe_path: Option<String>,
	pub current_live_digest: Option<String>,
}

pub struct DeploymentDeployHistory {
	pub image_digest: String,
	pub created: DateTime<Utc>,
}

pub struct DeploymentEnvironmentVariable {
	pub deployment_id: Uuid,
	pub name: String,
	pub value: Option<String>,
	pub secret_id: Option<Uuid>,
}

pub struct DeploymentConfigMount {
	pub path: String,
	pub file: Vec<u8>,
	pub deployment_id: Uuid,
}
pub struct DeploymentVolume {
	pub volume_id: Uuid,
	pub deployment_id: Uuid,
	pub size: i32,
	pub path: String,
}

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployments tables");

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_STATUS AS ENUM(
			'created', /* Created, but nothing pushed to it yet */
			'pushed', /* Something is pushed, but the system has not deployed it yet */
			'deploying', /* Something is pushed, and the system is currently deploying it */
			'running', /* Deployment is running successfully */
			'stopped', /* Deployment is stopped by the user */
			'errored', /* Deployment is stopped because of too many errors */
			'deleted' /* Deployment is deleted by the user */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_machine_type(
			id UUID CONSTRAINT deployment_machint_type_pk PRIMARY KEY,
			cpu_count SMALLINT NOT NULL,
			memory_count INTEGER NOT NULL /* Multiples of 0.25 GB */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE EXPOSED_PORT_TYPE AS ENUM(
			'http'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment(
			id UUID CONSTRAINT deployment_pk PRIMARY KEY,
			name CITEXT NOT NULL CONSTRAINT deployment_chk_name_is_trimmed
				CHECK(
					name = TRIM(name)
				),
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.patr.cloud',
			repository_id UUID,
			image_name VARCHAR(512) CONSTRAINT deployment_chk_image_name_is_valid
				CHECK(
					image_name ~ '^(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?)(((\/)(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?))*)?$'
				),
			image_tag VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
			region UUID NOT NULL CONSTRAINT deployment_fk_region
				REFERENCES deployment_region(id),
			min_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
					min_horizontal_scale >= 0 AND min_horizontal_scale <= 256
				)
				DEFAULT 1,
			max_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
					max_horizontal_scale >= 0 AND max_horizontal_scale <= 256
				)
				DEFAULT 1,
			machine_type UUID NOT NULL CONSTRAINT deployment_fk_machine_type
				REFERENCES deployment_machine_type(id),
			deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE,
			startup_probe_port INTEGER,
			startup_probe_path VARCHAR(255),
			startup_probe_port_type EXPOSED_PORT_TYPE,
			liveness_probe_port INTEGER,
			liveness_probe_path VARCHAR(255),
			liveness_probe_port_type EXPOSED_PORT_TYPE,
			current_live_digest TEXT,
			deleted TIMESTAMPTZ,
			CONSTRAINT deployment_fk_repository_id_workspace_id
				FOREIGN KEY(repository_id, workspace_id)
					REFERENCES docker_registry_repository(id, workspace_id),
			CONSTRAINT deployment_uq_id_workspace_id
				UNIQUE(id, workspace_id),
			CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
				(
					registry = 'registry.patr.cloud' AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				) OR (
					registry != 'registry.patr.cloud' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			),
			CONSTRAINT deployment_chk_image_tag_is_valid CHECK(
				image_tag != ''
			),
			CONSTRAINT deployment_chk_startup_probe_is_valid CHECK(
					(
						startup_probe_port IS NULL AND
						startup_probe_path IS NULL AND
						startup_probe_port_type IS NULL
					) OR (
						startup_probe_port IS NOT NULL AND
						startup_probe_path IS NOT NULL AND
						startup_probe_port_type IS NOT NULL
					)
				),
			CONSTRAINT deployment_chk_liveness_probe_is_valid CHECK(
					(
						liveness_probe_port IS NULL AND
						liveness_probe_path IS NULL AND
						liveness_probe_port_type IS NULL
					) OR (
						liveness_probe_port IS NOT NULL AND
						liveness_probe_path IS NOT NULL AND
						liveness_probe_port_type IS NOT NULL
					)
				),
			CONSTRAINT deployment_chk_startup_probe_port_type_is_http CHECK(
					startup_probe_port_type = 'http'
				),
			CONSTRAINT deployment_chk_liveness_probe_port_type_is_http CHECK(
					liveness_probe_port_type = 'http'
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_name
		ON
			deployment
		(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_image_name_image_tag
		ON
			deployment
		(image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_registry_image_name_image_tag
		ON
			deployment
		(registry, image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id UUID
				CONSTRAINT deployment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT,
			secret_id UUID,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name),
			CONSTRAINT deployment_environment_variable_fk_secret_id
				FOREIGN KEY(secret_id) REFERENCES secret(id),
			CONSTRAINT deployment_env_var_chk_value_secret_id_either_not_null
				CHECK(
					(
						value IS NOT NULL AND
						secret_id IS NULL
					) OR (
						value IS NULL AND
						secret_id IS NOT NULL
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_exposed_port(
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_exposed_port_fk_deployment_id
					REFERENCES deployment(id),
			port INTEGER NOT NULL CONSTRAINT
				deployment_exposed_port_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			port_type EXPOSED_PORT_TYPE NOT NULL,
			CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port),
			CONSTRAINT deployment_exposed_port_uq_deployment_id_port_port_type
				UNIQUE(deployment_id, port, port_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			ADD CONSTRAINT deployment_fk_deployment_id_startup_port_startup_port_type
				FOREIGN KEY (id, startup_probe_port, startup_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_fk_deployment_id_liveness_port_liveness_port_type
				FOREIGN KEY (id, liveness_probe_port, liveness_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_config_mounts(
			path TEXT NOT NULL
				CONSTRAINT deployment_config_mounts_chk_path_valid
					CHECK(path ~ '^[a-zA-Z0-9_\-\.\(\)]+$'),
			file BYTEA NOT NULL,
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_config_mounts_fk_deployment_id
					REFERENCES deployment(id),
			CONSTRAINT deployment_config_mounts_pk PRIMARY KEY(
				deployment_id,
				path
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_deploy_history(
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_deployment_id
					REFERENCES deployment(id),
			image_digest TEXT NOT NULL,
			repository_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_repository_id
					REFERENCES docker_registry_repository(id),
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT deployment_image_digest_pk
				PRIMARY KEY(deployment_id, image_digest)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_volume(
			id UUID CONSTRAINT deployment_volume_pk PRIMARY KEY,
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_volume_fk_deployment_id
					REFERENCES deployment(id),
			volume_size BIGINT NOT NULL
				CONSTRAINT
					deployment_volume_chk_size_unsigned
						CHECK(volume_size > 0),
			volume_mount_path TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_current_live_digest
		FOREIGN KEY(id, current_live_digest) REFERENCES
		deployment_deploy_history(deployment_id, image_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			deployment_uq_workspace_id_name
		ON
			deployment(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	for (cpu_count, memory_count) in DEFAULT_MACHINE_TYPES {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				deployment_machine_type(
					id,
					cpu_count,
					memory_count
				)
			VALUES
				($1, $2, $3);
			"#,
			machine_type_id as _,
			cpu_count,
			memory_count
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	repository_id: &Uuid,
	image_tag: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	startup_probe: Option<&DeploymentProbe>,
	liveness_probe: Option<&DeploymentProbe>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				repository_id,
				image_name,
				image_tag,
				status,
				workspace_id,
				region,
				min_horizontal_scale,
				max_horizontal_scale,
				machine_type,
				deploy_on_push,
				startup_probe_port,
				startup_probe_path,
				startup_probe_port_type,
				liveness_probe_port,
				liveness_probe_path,
				liveness_probe_port_type
			)
		VALUES
			(
				$1,
				$2,
				'registry.patr.cloud',
				$3,
				NULL,
				$4,
				'created',
				$5,
				$6,
				$7,
				$8,
				$9,
				$10,
				$11,
				$12,
				$13,
				$14,
				$15,
				$16
				
			);
		"#,
		id as _,
		name as _,
		repository_id as _,
		image_tag,
		workspace_id as _,
		region as _,
		min_horizontal_scale as i32,
		max_horizontal_scale as i32,
		machine_type as _,
		deploy_on_push,
		startup_probe.map(|probe| probe.port as i32),
		startup_probe.map(|probe| probe.path.as_str()),
		startup_probe.map(|_| ExposedPortType::Http) as _,
		liveness_probe.map(|probe| probe.port as i32),
		liveness_probe.map(|probe| probe.path.as_str()),
		liveness_probe.map(|_| ExposedPortType::Http) as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_deployment_with_external_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	startup_probe: Option<&DeploymentProbe>,
	liveness_probe: Option<&DeploymentProbe>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				repository_id,
				image_name,
				image_tag,
				status,
				workspace_id,
				region,
				min_horizontal_scale,
				max_horizontal_scale,
				machine_type,
				deploy_on_push,
				startup_probe_port,
				startup_probe_path,
				startup_probe_port_type,
				liveness_probe_port,
				liveness_probe_path,
				liveness_probe_port_type
			)
		VALUES
			(
				$1,
				$2,
				$3,
				NULL,
				$4,
				$5,
				'created',
				$6,
				$7,
				$8,
				$9,
				$10,
				$11,
				$12,
				$13,
				$14,
				$15,
				$16,
				$17
			);
		"#,
		id as _,
		name as _,
		registry,
		image_name,
		image_tag,
		workspace_id as _,
		region as _,
		min_horizontal_scale as i32,
		max_horizontal_scale as i32,
		machine_type as _,
		deploy_on_push,
		startup_probe.map(|probe| probe.port as i32),
		startup_probe.map(|probe| probe.path.as_str()),
		startup_probe.map(|_| ExposedPortType::Http) as _,
		liveness_probe.map(|probe| probe.port as i32),
		liveness_probe.map(|probe| probe.path.as_str()),
		liveness_probe.map(|_| ExposedPortType::Http) as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	image_name: &str,
	image_tag: &str,
	workspace_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id as "id: _",
			deployment.name::TEXT as "name!: _",
			deployment.registry,
			deployment.repository_id as "repository_id: _",
			deployment.image_name,
			deployment.image_tag,
			deployment.status as "status: _",
			deployment.workspace_id as "workspace_id: _",
			deployment.region as "region: _",
			deployment.min_horizontal_scale,
			deployment.max_horizontal_scale,
			deployment.machine_type as "machine_type: _",
			deployment.deploy_on_push,
			deployment.startup_probe_port,
			deployment.startup_probe_path,
			deployment.liveness_probe_port,
			deployment.liveness_probe_path,
			current_live_digest
		FROM
			deployment
		LEFT JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = deployment.repository_id
		WHERE
			(
				(
					deployment.registry = 'registry.patr.cloud' AND
					docker_registry_repository.name = $1
				) OR
				(
					deployment.registry != 'registry.patr.cloud' AND
					deployment.image_name = $1
				)
			) AND
			deployment.image_tag = $2 AND
			deployment.workspace_id = $3 AND
			deployment.status != 'deleted';
		"#,
		image_name as _,
		image_tag,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployments_by_repository_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			repository_id = $1 AND
			status != 'deleted';
		"#,
		repository_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployments_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Option<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_deployment_by_id_including_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Option<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_deployment_by_name_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &Uuid,
) -> Result<Option<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
		FROM
			deployment
		WHERE
			name = $1 AND
			workspace_id = $2 AND
			status != 'deleted';
		"#,
		name as _,
		workspace_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		deployment_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			deleted = $2,
			status = 'deleted'
		WHERE
			id = $1;
		"#,
		deployment_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<DeploymentEnvironmentVariable>, sqlx::Error> {
	query_as!(
		DeploymentEnvironmentVariable,
		r#"
		SELECT
		    deployment_id as "deployment_id: _",
			name,
			value,
			secret_id as "secret_id: _" 
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	key: &str,
	value: Option<&str>,
	secret_id: Option<&Uuid>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable(
				deployment_id,
				name,
				value,
				secret_id
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		deployment_id as _,
		key,
		value,
		secret_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_with_secret_as_environment_variable(
	connection: &mut <Database as sqlx::Database>::Connection,
	secret_id: &Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT DISTINCT
			deployment_id as "deployment_id: Uuid"
		FROM
			deployment_environment_variable
		LEFT JOIN
			deployment
		ON
			deployment.id = deployment_id
		WHERE
			secret_id = $1 AND
			deployment.deleted IS NULL AND
			deployment.status != 'deleted';
		"#,
		secret_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.deployment_id)
	.collect();

	Ok(rows)
}

pub async fn get_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<(u16, ExposedPortType)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			port,
			port_type as "port_type: ExposedPortType"
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.port as u16, row.port_type))
	.collect();

	Ok(rows)
}

pub async fn add_exposed_port_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	port: u16,
	exposed_port_type: &ExposedPortType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_exposed_port(
				deployment_id,
				port,
				port_type
			)
		VALUES
			($1, $2, $3);
		"#,
		deployment_id as _,
		port as i32,
		exposed_port_type as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	name: Option<&str>,
	machine_type: Option<&Uuid>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	startup_probe: Option<&DeploymentProbe>,
	liveness_probe: Option<&DeploymentProbe>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = COALESCE($1, name),
			machine_type = COALESCE($2, machine_type),
			deploy_on_push = COALESCE($3, deploy_on_push),
			min_horizontal_scale = COALESCE($4, min_horizontal_scale),
			max_horizontal_scale = COALESCE($5, max_horizontal_scale),
			startup_probe_port = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$6
				END
			),
			startup_probe_path = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$7
				END
			),
			startup_probe_port_type = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					WHEN $6 IS NULL THEN
						startup_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			),
			liveness_probe_port = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$8
				END
			),
			liveness_probe_path = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$9
				END
			),
			liveness_probe_port_type = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					WHEN $8 IS NULL THEN
						liveness_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			)
		WHERE
			id = $10;
		"#,
		name as _,
		machine_type as _,
		deploy_on_push,
		min_horizontal_scale.map(|v| v as i16),
		max_horizontal_scale.map(|v| v as i16),
		startup_probe.map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		deployment_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_config_mount_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	path: &str,
	file: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_config_mounts(
				path,
				file,
				deployment_id
			)
		VALUES
			($1, $2, $3);
		"#,
		path,
		file,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_volume_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	volume_id: &Uuid,
	size: u64,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_volume(
				id,
				volume_size,
				volume_mount_path,
				deployment_id
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		volume_id as _,
		size as i64,
		path,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_volume_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	volume_id: &Uuid,
	size: &u32,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE 
			deployment_volume
		SET
			volume_size = $1,
			volume_mount_path = $2
		WHERE
			id = $3 AND
			deployment_id = $4
		"#,
		*size as i32,
		path,
		volume_id as _,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_volume_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	volume_id: &Uuid,
) -> Result<Option<DeploymentVolume>, sqlx::Error> {
	query_as!(
		DeploymentVolume,
		r#"
		SELECT 
			id as "volume_id: _",
			deployment_id as "deployment_id: _",
			volume_size as "size: _",
			volume_mount_path as "path: _"
		FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		volume_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_deployment_volumes(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<DeploymentVolume>, sqlx::Error> {
	query_as!(
		DeploymentVolume,
		r#"
		SELECT
			id as "volume_id: _",
			deployment_id as "deployment_id: _",
			volume_size as "size: _",
			volume_mount_path as "path: _"
		FROM
			deployment_volume
		WHERE
			id = $1;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_deployment_config_mounts(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<DeploymentConfigMount>, sqlx::Error> {
	query_as!(
		DeploymentConfigMount,
		r#"
		SELECT
			deployment_id as "deployment_id: _",
			path,
			file as "file!: _"
		FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn remove_all_config_mounts_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_deployment_machine_types(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<DeploymentMachineType>, sqlx::Error> {
	query_as!(
		DeploymentMachineType,
		r#"
		SELECT
			id as "id: _",
			cpu_count,
			memory_count
		FROM
			deployment_machine_type;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_build_events_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<WorkspaceAuditLog>, sqlx::Error> {
	query_as!(
		WorkspaceAuditLog,
		r#"
		SELECT
			workspace_audit_log.id as "id: _",
			date as "date: _",
			ip_address,
			workspace_id as "workspace_id: _",
			user_id as "user_id: _",
			login_id as "login_id: _",
			resource_id as "resource_id: _",
			permission.name as "action",
			request_id as "request_id: _",
			metadata as "metadata: _",
			patr_action as "patr_action: _",
			success
		FROM
			workspace_audit_log
		INNER JOIN
			permission
		ON
			permission.id = workspace_audit_log.action
		WHERE
			resource_id = $1 AND 
			(
				metadata ->> 'action' = 'create' OR
				metadata ->> 'action' = 'start' OR
				metadata ->> 'action' = 'stop' OR
				metadata ->> 'action' = 'updateImage'
			);
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_digest_to_deployment_deploy_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	repository_id: &Uuid,
	digest: &str,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_deploy_history(
				deployment_id,
				image_digest,
				repository_id,
				created
			)
		VALUES
			($1, $2, $3, $4)
		ON CONFLICT
			(deployment_id, image_digest)
		DO NOTHING;
		"#,
		deployment_id as _,
		digest as _,
		repository_id as _,
		created as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_current_live_digest_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	digest: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			current_live_digest = $1
		WHERE
			id = $2;
		"#,
		digest as _,
		deployment_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_image_digest_by_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	digest: &str,
) -> Result<Option<DeploymentDeployHistory>, sqlx::Error> {
	query_as!(
		DeploymentDeployHistory,
		r#"
		SELECT 
			image_digest,
			created as "created: _"
		FROM
			deployment_deploy_history
		WHERE
			image_digest = $1;
		"#,
		digest as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_digest_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<DeploymentDeployHistory>, sqlx::Error> {
	query_as!(
		DeploymentDeployHistory,
		r#"
		SELECT 
			image_digest,
			created as "created: _"
		FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}
