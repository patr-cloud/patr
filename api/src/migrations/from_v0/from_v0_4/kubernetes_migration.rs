use std::pin::Pin;

use api_macros::query_as;
use api_models::utils::Uuid;
use futures::Future;
use sqlx::Row;

use crate::{
	migrate_query as query,
	models::db_mapping::DeploymentRegion,
	utils::settings::Settings,
	Database,
};

#[derive(sqlx::Type, Debug, Clone, PartialEq)]
#[sqlx(type_name = "DEPLOYMENT_CLOUD_PROVIDER", rename_all = "lowercase")]
enum DeploymentCloudProvider {
	Digitalocean,
}

lazy_static::lazy_static! {
	static ref DEFAULT_DEPLOYMENT_REGIONS: Vec<DefaultDeploymentRegion> = vec![
		DefaultDeploymentRegion {
			name: "Asia",
			cloud_provider: None,
			coordinates: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Singapore",
					cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
					coordinates: Some((1.3521, 103.8198)),
					child_regions: vec![],
				},
				DefaultDeploymentRegion {
					name: "India",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Bangalore",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((2.9716, 77.5946)),
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "Europe",
			cloud_provider: None,
			coordinates: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "England",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "London",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((51.5072, 0.1276)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Netherlands",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Amsterdam",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((52.3676, 4.9041)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Germany",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Frankfurt",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((50.1109, 8.6821)),
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "North-America",
			cloud_provider: None,
			coordinates: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Canada",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Toronto",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((43.6532, 79.3832)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "USA",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![
						DefaultDeploymentRegion {
							name: "New-York 1",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "New-York 2",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "San Francisco",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((37.7749, 122.4194)),
							child_regions: vec![],
						},
					],
				},
			],
		},
	];
}

#[derive(Debug, Clone)]
struct DefaultDeploymentRegion {
	pub name: &'static str,
	pub cloud_provider: Option<DeploymentCloudProvider>,
	pub coordinates: Option<(f64, f64)>,
	pub child_regions: Vec<DefaultDeploymentRegion>,
}

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
	for region in DEFAULT_DEPLOYMENT_REGIONS.iter() {
		populate_region(&mut *connection, None, region.clone()).await?;
	}

	Ok(())
}

fn populate_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	parent_region_id: Option<Uuid>,
	region: DefaultDeploymentRegion,
) -> Pin<Box<dyn Future<Output = Result<(), sqlx::Error>>>> {
	Box::pin(async {
		let region_id = loop {
			let region_id = Uuid::new_v4();

			let row = query!(
				r#"
				SELECT
					id as "id: Uuid"
				FROM
					deployment_region
				WHERE
					id = $1;
				"#,
				&region_id
			)
			.fetch_optional(&mut *connection)
			.await?;

			if row.is_none() {
				break region_id;
			}
		};

		if region.child_regions.is_empty() {
			// Populate leaf node
			query!(
				r#"
				INSERT INTO
					deployment_region
				VALUES
					($1, $2, $3, ST_SetSRID(POINT($4, $5)::GEOMETRY, 4326), $6);
				"#,
				&region_id,
				&region.name,
				region.cloud_provider.unwrap(),
				region.coordinates.unwrap().0,
				region.coordinates.unwrap().1,
				&parent_region_id,
			)
			.execute(&mut *connection)
			.await?;
		} else {
			// Populate parent node
			query!(
				r#"
				INSERT INTO
					deployment_region
				VALUES
					($1, $2, NULL, NULL, $3);
				"#,
				&region_id,
				&region.name,
				&parent_region_id,
			)
			.execute(&mut *connection)
			.await?;

			for child_region in region.child_regions {
				populate_region(
					&mut *connection,
					Some(region_id.clone()),
					child_region,
				)
				.await?;
			}
		}

		Ok(())
	})
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
		client
			.delete(format!("https://api.digitalocean.com/v2/apps/{}", app_id))
			.bearer_auth(&config.digitalocean.api_key)
			.send()
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?
			.status();
	}

	Ok(())
}

async fn migrate_all_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	todo!()
}

async fn get_id_by_region_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_name: &str,
) -> Result<DeploymentRegion, sqlx::Error> {
	let region = query_as!(
		DeploymentRegion,
		r#"
		SELECT
			id as "id!: _",
			name as "name!: String",
			provider as "provider: _",
			location as "location: _",
			parent_region_id as "parent_region_id: _"
		FROM
			deployment_region
		WHERE
			name = $1;
		"#,
		region_name
	)
	.fetch_one(&mut *connection)
	.await?;

	Ok(region)
}
