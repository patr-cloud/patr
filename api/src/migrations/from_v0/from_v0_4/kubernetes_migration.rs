use std::time::{SystemTime, UNIX_EPOCH};

use api_models::utils::Uuid;
use sqlx::Row;

use crate::{migrate_query as query, utils::settings::Settings, Database};

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
	migrate_regions(&mut *connection, config).await?;
	add_exposed_port(&mut *connection, config).await?;
	update_deployments_table(&mut *connection, config).await?;
	migrate_machine_types(&mut *connection, config).await?;
	add_deploy_on_push(&mut *connection, config).await?;
	stop_all_running_deployments(&mut *connection, config).await?;
	delete_all_do_apps(&mut *connection, config).await?;
	// Make sure this is done for both static sites AND deployments
	migrate_all_managed_urls(&mut *connection, config).await?;
	remove_outdated_tables(&mut *connection, config).await?;
	rename_permissions_to_managed_url(&mut *connection, config).await?;
	rename_resource_type_to_managed_url(&mut *connection, config).await?;

	Ok(())
}

async fn migrate_regions(
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
		ALTER TABLE deployment ADD COLUMN region_new UUID
		CONSTRAINT deployment_fk_region
			REFERENCES deployment_region(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for continent in DEFAULT_DEPLOYMENT_REGIONS.iter() {
		let region_id =
			populate_region(&mut *connection, None, continent).await?;
		for country in continent.child_regions.iter() {
			let region_id =
				populate_region(&mut *connection, Some(&region_id), country)
					.await?;
			for city in country.child_regions.iter() {
				populate_region(&mut *connection, Some(&region_id), city)
					.await?;
			}
		}
	}

	const TO_AND_FROM_REGIONS: [(&str, &str); 8] = [
		("do-sgp", "Singapore"),
		("do-blr", "Bangalore"),
		("do-lon", "London"),
		("do-ams", "Amsterdam"),
		("do-fra", "Frankfurt"),
		("do-tor", "Toronto"),
		("do-nyc", "New-York 1"),
		("do-sfo", "San Francisco"),
	];
	for (from, to) in TO_AND_FROM_REGIONS {
		query!(
			r#"
			UPDATE
				deployment
			SET
				region_new = (
					SELECT
						id
					FROM
						deployment_region
					WHERE
						name = $1
				)
			WHERE
				region = $2;
			"#,
			to,
			from
		)
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(&format!(
		r#"
			UPDATE
				deployment
			SET
				region_new = (
					SELECT
						id
					FROM
						deployment_region
					WHERE
						name = $1
				)
			WHERE
				region NOT IN (
					{}
				);
			"#,
		TO_AND_FROM_REGIONS
			.iter()
			.map(|(from, _)| format!("'{}'", from))
			.collect::<Vec<_>>()
			.join(", ")
	))
	.bind("Bangalore")
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		DROP COLUMN region;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		RENAME COLUMN region_new TO region;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN region SET NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn populate_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	parent_region_id: Option<&Uuid>,
	region: &DefaultDeploymentRegion,
) -> Result<Uuid, sqlx::Error> {
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
			region.cloud_provider.as_ref().unwrap(),
			region.coordinates.unwrap().0,
			region.coordinates.unwrap().1,
			parent_region_id,
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
			parent_region_id,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(region_id)
}

async fn add_exposed_port(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
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
			port INTEGER NOT NULL CONSTRAINT
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

	query!(
		r#"
		INSERT INTO
			deployment_exposed_port
			(
				SELECT
					deployment.id,
					80 as "port",
					'http' as "port_type"
				FROM
					deployment
				WHERE
					deployment.status != 'deleted'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_deployments_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
			ADD COLUMN min_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
					min_horizontal_scale >= 0 AND min_horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN max_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
					max_horizontal_scale >= 0 AND max_horizontal_scale <= 256
				)
				DEFAULT 1;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			min_horizontal_scale = horizontal_scale,
			max_horizontal_scale = horizontal_scale;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		DROP COLUMN horizontal_scale;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			DROP CONSTRAINT deployment_fk_repository_id,
			ADD CONSTRAINT deployment_fk_repository_id_workspace_id
				FOREIGN KEY (repository_id, workspace_id)
					REFERENCES docker_registry_repository(id, workspace_id);
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
		ALTER TABLE deployment ADD COLUMN machine_type_new UUID
		CONSTRAINT deployment_fk_machine_type
			REFERENCES deployment_machine_type_new(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

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
				deployment_machine_type_new
			VALUES
				($1, $2, $3);
			"#,
			machine_type_id,
			cpu_count,
			memory_count
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
					deployment_machine_type_new
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
					deployment_machine_type_new
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
					deployment_machine_type_new
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
					deployment_machine_type_new
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
					deployment_machine_type_new
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

	query!(
		r#"
		ALTER TABLE deployment
		DROP COLUMN machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		RENAME COLUMN machine_type_new TO machine_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN machine_type SET NOT NULL;
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

async fn add_deploy_on_push(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE;
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
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

	query!(
		r#"
		ALTER TABLE deployment
			DROP COLUMN deployed_image,
			DROP COLUMN digitalocean_app_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_all_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		DROP COLUMN domain_name CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		DROP COLUMN domain_name CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_URL_TYPE AS ENUM(
			'proxy_to_deployment',
			'proxy_to_static_site',
			'proxy_url',
			'redirect'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url(
			id UUID CONSTRAINT managed_url_pk PRIMARY KEY,
			sub_domain TEXT NOT NULL
				CONSTRAINT managed_url_chk_sub_domain_valid CHECK(
					sub_domain ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-\.]*[a-z0-9]))$' OR
					sub_domain LIKE CONCAT(
						'^patr-deleted\: ',
						REPLACE(id::TEXT, '-', ''),
						'@%'
					)
				),
			domain_id UUID NOT NULL,
			path TEXT NOT NULL,
			url_type MANAGED_URL_TYPE NOT NULL,
			deployment_id UUID,
			port INTEGER CONSTRAINT managed_url_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			static_site_id UUID,
			url TEXT,
			workspace_id UUID NOT NULL,
			CONSTRAINT managed_url_uq_sub_domain_domain_id_path UNIQUE(
				sub_domain, domain_id, path
			),
			CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
				(
					url_type = 'proxy_to_deployment' AND
					deployment_id IS NOT NULL AND
					port IS NOT NULL AND
					static_site_id IS NULL AND
					url IS NULL
				) OR
				(
					url_type = 'proxy_to_static_site' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NOT NULL AND
					url IS NULL
				) OR
				(
					url_type = 'proxy_url' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NULL AND
					url IS NOT NULL
				) OR
				(
					url_type = 'redirect' AND
					deployment_id IS NULL AND
					port IS NULL AND
					static_site_id IS NULL AND
					url IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_uq_id_workspace_id
		UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		ADD CONSTRAINT deployment_static_site_uq_id_workspace_id
		UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES workspace_domain(id),
			ADD CONSTRAINT managed_url_fk_domain_id_workspace_id
				FOREIGN KEY(domain_id, workspace_id)
					REFERENCES resource(id, owner_id),
			ADD CONSTRAINT managed_url_fk_deployment_id_port
				FOREIGN KEY(deployment_id, port)
					REFERENCES deployment_exposed_port(deployment_id, port),
			ADD CONSTRAINT managed_url_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id),
			ADD CONSTRAINT managed_url_fk_static_site_id_workspace_id
				FOREIGN KEY(static_site_id, workspace_id)
					REFERENCES deployment_static_site(id, workspace_id),
			ADD CONSTRAINT managed_url_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id)
					REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add all deployed_domains as user-controlled, unverified domains
	let deployed_domains = query!(
		r#"
		SELECT
			*
		FROM
			deployed_domain;
		"#
	)
	.fetch_all(&mut *connection)
	.await?;

	let tld_list =
		reqwest::get("https://data.iana.org/TLD/tlds-alpha-by-domain.txt")
			.await
			.map_err(|_| sqlx::Error::WorkerCrashed)?
			.text()
			.await
			.map_err(|_| sqlx::Error::WorkerCrashed)?
			.split('\n')
			.map(String::from)
			.filter(|tld| {
				!tld.starts_with('#') &&
					!tld.is_empty() && !tld.starts_with("XN--")
			})
			.collect::<Vec<String>>();

	let domain_resource_type_id = query!(
		r#"
		SELECT
			id as "id!: Uuid"
		FROM
			resource_type
		WHERE
			name = 'domain';
		"#
	)
	.fetch_one(&mut *connection)
	.await?
	.get::<Uuid, _>("id");

	let managed_url_resource_type_id = query!(
		r#"
		SELECT
			id as "id!: Uuid"
		FROM
			resource_type
		WHERE
			name = 'deploymentEntryPoint';
		"#
	)
	.fetch_one(&mut *connection)
	.await?
	.get::<Uuid, _>("id");

	for deployed_domain in deployed_domains {
		let raw_domain_name = deployed_domain.get::<String, _>("domain_name");
		let (deployment_id, static_site_id) = (
			deployed_domain.get::<Option<Uuid>, _>("deployment_id"),
			deployed_domain.get::<Option<Uuid>, _>("static_site_id"),
		);

		let workspace_id = if let Some(deployment_id) = &deployment_id {
			query!(
				r#"
				SELECT
					workspace_id
				FROM
					deployment
				WHERE
					id = $1;
				"#,
				deployment_id
			)
			.fetch_one(&mut *connection)
			.await?
			.get::<Uuid, _>("workspace_id")
		} else {
			query!(
				r#"
				SELECT
					workspace_id
				FROM
					deployment_static_site
				WHERE
					id = $1;
				"#,
				static_site_id.as_ref().unwrap()
			)
			.fetch_one(&mut *connection)
			.await?
			.get::<Uuid, _>("workspace_id")
		};

		let deleted_marker = format!(
			"deleted.patr.cloud.{}.",
			if deployment_id.is_some() {
				deployment_id.as_ref()
			} else {
				static_site_id.as_ref()
			}
			.unwrap()
		);

		let tld = tld_list
			.iter()
			.filter(|tld| raw_domain_name.ends_with(*tld))
			.reduce(|accumulator, item| {
				if accumulator.len() > item.len() {
					accumulator
				} else {
					item
				}
			});
		let tld = if let Some(tld) = tld {
			tld
		} else {
			// TLD does not exist. Incompatible domain. Skip
			continue;
		};

		let domain_name = raw_domain_name
			.to_lowercase()
			.replace(&deleted_marker, "")
			.replace(&format!(".{}", tld), "");

		if raw_domain_name.starts_with(&deleted_marker) {
			// TODO Domain is deleted. Handle that separately (future work which
			// can be done manually)
			continue;
		}

		// Check if the domain already exists
		let domain_id = query!(
			r#"
			SELECT
				id
			FROM
				domain
			WHERE
				name = $1;
			"#,
			&format!("{}.{}", domain_name, tld)
		)
		.fetch_optional(&mut *connection)
		.await?
		.map(|row| row.get::<Uuid, _>("id"));

		let managed_url_id = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					managed_url
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

		let domain_id = if let Some(domain_id) = domain_id {
			// Domain already exists. Use this to add the managed_url
			domain_id
		} else {
			// Domain does not exist. Create it
			let domain_id = loop {
				let uuid = Uuid::new_v4();

				let exists = query!(
					r#"
					SELECT
						*
					FROM
						workspace_domain
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

			let current_time = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.expect("Time went backwards. Wtf?")
				.as_millis() as i64;

			// create resource
			query!(
				r#"
				INSERT INTO
					resource
				VALUES
					($1, $2, $3, $4, $5);
				"#,
				&domain_id,
				&format!("Domain: {}", domain_name),
				&domain_resource_type_id,
				&workspace_id,
				current_time
			)
			.execute(&mut *connection)
			.await?;

			query!(
				r#"
				INSERT INTO
					domain
				VALUES
					($1, $2, 'business', $3);
				"#,
				&domain_id,
				&domain_name,
				&tld
			)
			.execute(&mut *connection)
			.await?;

			query!(
				r#"
				INSERT INTO
					workspace_domain
				VALUES
					($1, 'business', FALSE, 'external');
				"#,
				&domain_id
			)
			.execute(&mut *connection)
			.await?;

			query!(
				r#"
				INSERT INTO
					user_controlled_domain
				VALUES
					($1, 'external');
				"#,
				&domain_id
			)
			.execute(&mut *connection)
			.await?;

			domain_id
		};

		let current_time = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards. Wtf?")
			.as_millis() as i64;

		// create resource
		query!(
			r#"
			INSERT INTO
				resource
			VALUES
				($1, $2, $3, $4, $5);
			"#,
			&domain_id,
			&format!("Managed URL: {}", managed_url_id),
			&managed_url_resource_type_id,
			&workspace_id,
			current_time
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			INSERT INTO
				managed_url
			VALUES
				(
					$1,
					$2,
					$3,
					"/",
					$4,
					$5,
					80,
					$6,
					NULL,
					$7
				);
			"#,
			&managed_url_id,
			&raw_domain_name
				.replace(&deleted_marker, "")
				.replace(&format!(".{}.{}", domain_name, tld), ""),
			&domain_id,
			"/",
			if deployment_id.is_some() {
				"proxy_to_deployment"
			} else {
				"proxy_to_static_site"
			},
			&deployment_id,
			&static_site_id,
			&workspace_id
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		DROP TABLE deployed_domain CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

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

async fn rename_permissions_to_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(name, 'entryPoint', 'managedUrl')
		WHERE
			name LIKE 'workspace::infrastructure::entryPoint::%';
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

async fn rename_resource_type_to_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			resource_type
		SET
			name = 'managedUrl'
		WHERE
			name = 'deploymentEntryPoint';
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
