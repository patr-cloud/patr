use api_models::utils::Uuid;
use sqlx::Row;

use crate::{migrate_query as query, utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	remove_outdated_tables(&mut *connection, config).await?;
	create_new_tables(&mut *connection, config).await?;

	migrate_machine_types(&mut *connection, config).await?;
	migrate_regions(&mut *connection, config).await?;
	stop_all_running_deployments(&mut *connection, config).await?;
	delete_all_do_apps(&mut *connection, config).await?;
	// Make sure this is done for both static sites AND deployments
	migrate_all_managed_urls(&mut *connection, config).await?;

	remove_old_columns(&mut *connection, config).await?;
	rename_new_columns_to_old_names(&mut *connection, config).await?;

	Ok(())
}

async fn remove_outdated_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	// Remove old tables.
	query!(
		r#"
		DROP TABLE data_center_locations;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO Move deployed_domain to managed_urls:
	// Find the domains in deployed_domain.
	// Create those domains as user-controlled domains, but unverified.
	// Add them all as proxy_deployment or proxy_static_site to managed_urls.

	query!(
		r#"
		DROP TABLE deployed_domain CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE deployment_request_logs;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_REQUEST_METHOD;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_REQUEST_PROTOCOL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_new_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TYPE DEPLOYMENT_CLOUD_PROVIDER AS ENUM(
			'digitalocean'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider DEPLOYMENT_CLOUD_PROVIDER,
			location GEOMETRY,
			parent_region_id UUID
				CONSTRAINT deployment_region_fk_parent_region_id
					REFERENCES deployment_region(id),
			CONSTRAINT
				deployment_region_chk_provider_location_parent_region_is_valid
				CHECK(
					(
						location IS NULL AND
						provider IS NULL
					) OR
					(
						provider IS NOT NULL AND
						location IS NOT NULL AND
						parent_region_id IS NOT NULL
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_machine_type_new(
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
		ALTER TABLE deployment
			ADD COLUMN region UUID NOT NULL CONSTRAINT deployment_fk_region
				REFERENCES deployment_region(id),
			ADD COLUMN min_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
					min_horizontal_scale >= 0 AND min_horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN max_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
					max_horizontal_scale >= 0 AND max_horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN machine_type_new UUID NOT NULL
				CONSTRAINT deployment_fk_machine_type
					REFERENCES deployment_machine_type_new(id),
			ADD COLUMN deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE;
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
		CREATE TABLE deployment_exposed_port(
			deployment_id UUID
				CONSTRAINT deployment_exposed_port_fk_deployment_id
					REFERENCES deployment(id),
			port SMALLINT NOT NULL CONSTRAINT
				deployment_exposed_port_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			port_type EXPOSED_PORT_TYPE NOT NULL,
			CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_machine_types(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	// Create machine types
	for (cpu_count, memory_count) in [
		(1, 2),  // 1 vCPU, 0.5 GB RAM
		(1, 4),  // 1 vCPU, 1 GB RAM
		(1, 8),  // 1 vCPU, 2 GB RAM
		(2, 8),  // 2 vCPU, 4 GB RAM
		(4, 32), // 4 vCPU, 8 GB RAM
	] {
		let machine_type_id = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					resource
				WHERE
					id = $1;
				"#,
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				break uuid;
			}
		};
		query!(
			r#"
			INSERT INTO
				deployment_machine_type
			VALUES
				($1, $2, $3);
			"#,
			machine_type_id,
			cpu_count as i32,
			memory_count as i32
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type_new = (
				SELECT
					id
				FROM
					deployment_machine_type
				WHERE
					cpu_count = 1 AND
					memory_count = 2
			)
		WHERE
			machine_type = 'micro';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type_new = (
				SELECT
					id
				FROM
					deployment_machine_type
				WHERE
					cpu_count = 1 AND
					memory_count = 2
			)
		WHERE
			machine_type = 'micro';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type_new = (
				SELECT
					id
				FROM
					deployment_machine_type
				WHERE
					cpu_count = 1 AND
					memory_count = 4
			)
		WHERE
			machine_type = 'small';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type_new = (
				SELECT
					id
				FROM
					deployment_machine_type
				WHERE
					cpu_count = 1 AND
					memory_count = 8
			)
		WHERE
			machine_type = 'medium';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			machine_type_new = (
				SELECT
					id
				FROM
					deployment_machine_type
				WHERE
					cpu_count = 2 AND
					memory_count = 16
			)
		WHERE
			machine_type = 'large';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_old_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
			DROP COLUMN deployed_image,
			DROP COLUMN digitalocean_app_id,
			DROP COLUMN region,
			DROP COLUMN domain_name CASCADE,
			DROP COLUMN horizontal_scale,
			DROP COLUMN machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		DROP COLUMN domain_name CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE DEPLOYMENT_MACHINE_TYPE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_new_columns_to_old_names(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_machine_type_new
		RENAME TO deployment_machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_regions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	todo!()
}

async fn stop_all_running_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = 'stopped'
		WHERE
			status = 'running' OR
			digitalocean_app_id IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn delete_all_do_apps(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	let app_ids = query!(
		r#"
		SELECT
			digitalocean_app_id
		FROM
			deployment
		WHERE
			digitalocean_app_id IS NOT NULL;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<String, _>("digitalocean_app_id"));

	let client = reqwest::Client::new();
	for app_id in app_ids {
		// delete DO app
		let response = client
			.delete(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
			.bearer_auth(&config.digitalocean.api_key)
			.send()
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
		if !response.status().is_success() {
			return Err(sqlx::Error::Protocol(format!(
				"failed to delete DO app: `{:#?}`",
				response.text().await
			)));
		}
	}

	Ok(())
}

async fn migrate_all_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	todo!()
}
