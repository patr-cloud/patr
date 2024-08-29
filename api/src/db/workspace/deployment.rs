use crate::prelude::*;

/// Initializes the deployment tables
#[instrument(skip(connection))]
pub async fn initialize_deployment_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment tables");
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
			id UUID NOT NULL,
			cpu_count SMALLINT NOT NULL,
			memory_count BIGINT NOT NULL /* Multiples of 0.25 GB */
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
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.patr.cloud',
			repository_id UUID,
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
			runner UUID NOT NULL,
			min_horizontal_scale SMALLINT NOT NULL DEFAULT 1,
			max_horizontal_scale SMALLINT NOT NULL DEFAULT 1,
			machine_type UUID NOT NULL,
			deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE,
			startup_probe_port INTEGER,
			startup_probe_path VARCHAR(255),
			startup_probe_port_type EXPOSED_PORT_TYPE,
			liveness_probe_port INTEGER,
			liveness_probe_path VARCHAR(255),
			liveness_probe_port_type EXPOSED_PORT_TYPE,
			current_live_digest TEXT,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id UUID NOT NULL,
			name VARCHAR(256) NOT NULL,
			value TEXT,
			secret_id UUID
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_exposed_port(
			deployment_id UUID NOT NULL,
			port INTEGER NOT NULL,
			port_type EXPOSED_PORT_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_config_mounts(
			path TEXT NOT NULL,
			file BYTEA NOT NULL,
			deployment_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_deploy_history(
			deployment_id UUID NOT NULL,
			image_digest TEXT NOT NULL,
			repository_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the deployment indices
#[instrument(skip(connection))]
pub async fn initialize_deployment_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment indices");
	query!(
		r#"
		ALTER TABLE deployment_machine_type
		ADD CONSTRAINT deployment_machine_type_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			ADD CONSTRAINT deployment_pk PRIMARY KEY(id),
			ADD CONSTRAINT deployment_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ADD CONSTRAINT deployment_environment_variable_pk
		PRIMARY KEY(deployment_id, name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_exposed_port
			ADD CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port),
			ADD CONSTRAINT deployment_exposed_port_uq_deployment_id_port_port_type
				UNIQUE(deployment_id, port, port_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_config_mounts
		ADD CONSTRAINT deployment_config_mounts_pk
		PRIMARY KEY(deployment_id, path);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_deploy_history
		ADD CONSTRAINT deployment_image_digest_pk
		PRIMARY KEY(deployment_id, image_digest);
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

	Ok(())
}

/// Initializes the deployment constraints
#[instrument(skip(connection))]
pub async fn initialize_deployment_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment constraints");
	query!(
		r#"
		ALTER TABLE deployment
			ADD CONSTRAINT deployment_chk_name_is_trimmed CHECK(
				name = TRIM(name)
			),
			ADD CONSTRAINT deployment_chk_image_name_is_valid CHECK(
				image_name ~ '^[a-zA-Z0-9\-_ \./]{4,255}$'
			),
			ADD CONSTRAINT deployment_fk_runner 
				FOREIGN KEY(runner) REFERENCES runner(id),
			ADD CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
				min_horizontal_scale >= 0 AND
				min_horizontal_scale <= 256 AND
				min_horizontal_scale <= max_horizontal_scale
			),
			ADD CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
				max_horizontal_scale >= 0 AND
				max_horizontal_scale <= 256 AND
				max_horizontal_scale >= min_horizontal_scale
			),
			ADD CONSTRAINT deployment_fk_machine_type
				FOREIGN KEY(machine_type) REFERENCES deployment_machine_type(id),
			ADD CONSTRAINT deployment_fk_repository_id_workspace_id
				FOREIGN KEY(repository_id, workspace_id)
					REFERENCES container_registry_repository(id, workspace_id),
			ADD CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
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
			ADD CONSTRAINT deployment_chk_image_tag_is_valid CHECK(
				image_tag != ''
			),
			ADD CONSTRAINT deployment_chk_startup_probe_is_valid CHECK(
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
			ADD CONSTRAINT deployment_chk_liveness_probe_is_valid CHECK(
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
			ADD CONSTRAINT deployment_chk_startup_probe_port_type_is_http CHECK(
				startup_probe_port_type = 'http'
			),
			ADD CONSTRAINT deployment_chk_liveness_probe_port_type_is_http CHECK(
				liveness_probe_port_type = 'http'
			),
			ADD CONSTRAINT deployment_fk_deployment_id_startup_port_startup_port_type
				FOREIGN KEY(id, startup_probe_port, startup_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
						DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_fk_deployment_id_liveness_port_liveness_port_type
				FOREIGN KEY(id, liveness_probe_port, liveness_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
						DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_fk_current_live_digest
				FOREIGN KEY(id, current_live_digest) REFERENCES
					deployment_deploy_history(deployment_id, image_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD CONSTRAINT deployment_environment_variable_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			ADD CONSTRAINT deployment_environment_variable_fk_secret_id
				FOREIGN KEY(secret_id) REFERENCES secret(id),
			ADD CONSTRAINT deployment_env_var_chk_value_secret_id_either_not_null CHECK(
				(
					value IS NOT NULL AND
					secret_id IS NULL
				) OR (
					value IS NULL AND
					secret_id IS NOT NULL
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_exposed_port
			ADD CONSTRAINT deployment_exposed_port_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			ADD CONSTRAINT deployment_exposed_port_chk_port_u16 CHECK(
				port > 0 AND port <= 65535
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_config_mounts
			ADD CONSTRAINT deployment_config_mounts_chk_path_valid
				CHECK(path ~ '^[a-zA-Z0-9_\-\.\(\)]+$'),
			ADD CONSTRAINT deployment_config_mounts_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_deploy_history
			ADD CONSTRAINT deployment_image_digest_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			ADD CONSTRAINT deployment_image_digest_fk_repository_id
				FOREIGN KEY(repository_id) REFERENCES container_registry_repository(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
