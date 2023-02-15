use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	reset_invalid_birthday(&mut *connection, config).await?;
	cloudflare_ingress::run_migrations(&mut *connection, config).await?;

	// Volumes migration
	add_volume_resource_type(&mut *connection, config).await?;
	add_volume_payment_history(&mut *connection, config).await?;
	add_deployment_volume_info(&mut *connection, config).await?;
	add_volume_storage_limit_to_workspace(&mut *connection, config).await?;
	add_sync_column(&mut *connection, config).await?;

	Ok(())
}

pub(super) async fn reset_invalid_birthday(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating all invalid user dob");

	query!(
		r#"
		UPDATE
			"user"
		SET
			dob = NULL
		WHERE
			dob IS NOT NULL AND
			dob > (NOW() - INTERVAL '13 YEARS');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	log::info!("All invalid dobs updated");

	query!(
		r#"
		ALTER TABLE "user" 
		ADD CONSTRAINT user_chk_dob_is_13_plus 
		CHECK (dob IS NULL OR dob < (NOW() - INTERVAL '13 YEARS'));             
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let resource_type_id = loop {
		let resource_type_id = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
			WHERE
				id = $1;
			"#,
			&resource_type_id
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break resource_type_id;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type(
				id,
				name,
				description
			)
		VALUES
			($1, 'deploymentVolume', '');
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// transaction table migrations
	query!(
		r#"
		CREATE TABLE volume_payment_history(
			workspace_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			number_of_volumes INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_deployment_volume_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE deployment_volume(
			id UUID CONSTRAINT deployment_volume_pk PRIMARY KEY,
			name TEXT NOT NULL,
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_volume_fk_deployment_id
					REFERENCES deployment(id),
			volume_size INT NOT NULL CONSTRAINT
				deployment_volume_chk_size_unsigned
					CHECK(volume_size > 0),
			volume_mount_path TEXT NOT NULL,
			deleted TIMESTAMPTZ,
			CONSTRAINT deployment_volume_name_unique_deployment_id
				UNIQUE(deployment_id, name),
			CONSTRAINT deployment_volume_path_unique_deployment_id
				UNIQUE(deployment_id, volume_mount_path)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_storage_limit_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN volume_storage_limit INTEGER NOT NULL DEFAULT 100;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN volume_storage_limit DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

mod cloudflare_ingress {
	use api_models::utils::Uuid;
	use cloudflare::{
		endpoints::{
			workers::{self, CreateRouteParams},
			zone::custom_hostname::{
				CreateCustomHostname,
				CreateCustomHostnameParams,
				SslParams,
				SslSettingsParams,
			},
		},
		framework::{
			async_api::Client as CloudflareClient,
			auth::Credentials,
			Environment,
			HttpApiClientConfig,
		},
	};
	use futures::TryStreamExt;
	use sqlx::Row;

	use crate::{
		migrate_query as query,
		utils::{settings::Settings, Error},
		Database,
	};

	pub async fn run_migrations(
		connection: &mut <Database as sqlx::Database>::Connection,
		config: &Settings,
	) -> Result<(), Error> {
		// todo: run migrations with original data in db
		log::info!("Running cloudflare ingress migrations");

		// non-idempotent migrations
		migrate_workspace_domain(connection, config).await?;
		migrate_managed_url(connection, config).await?;

		// idempotent migrations
		migrate_byoc_region(connection, config).await?;

		log::info!("Completed cloudflare ingress migrations");
		Ok(())
	}

	async fn migrate_workspace_domain(
		connection: &mut <Database as sqlx::Database>::Connection,
		config: &Settings,
	) -> Result<(), Error> {
		log::info!("Running workspace domain migrations for cf ingress");

		query!(
			r#"
			ALTER TABLE workspace_domain
				ADD COLUMN cf_route_id TEXT NOT NULL
					DEFAULT 'already_deleted';
			"#
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			ALTER TABLE workspace_domain
				ALTER COLUMN cf_route_id DROP DEFAULT;
			"#
		)
		.execute(&mut *connection)
		.await?;

		let valid_workspace_domain = query!(
			r#"
			SELECT
				workspace_domain.id as "domain_id",
				concat(domain.name, '.', domain.tld) AS "domain_name"
			FROM workspace_domain
			JOIN domain
				ON workspace_domain.id = domain.id
			WHERE domain.deleted IS NULL;
			"#
		)
		.fetch(&mut *connection)
		.map_ok(|row| {
			(
				row.get::<Uuid, _>("domain_id"),
				row.get::<String, _>("domain_name"),
			)
		})
		.try_collect::<Vec<_>>()
		.await?;

		for (idx, (domain_id, domain_name)) in
			valid_workspace_domain.iter().enumerate()
		{
			log::info!(
				"{}/{} - Creating cf worker routes for domain {} with name `{}`",
				idx,
				valid_workspace_domain.len(),
				domain_name,
				domain_id,
			);

			let cf_route_id =
				create_cf_worker_routes_for_domain(domain_name, config).await?;

			query!(
				r#"
				UPDATE
					workspace_domain
				SET
					cf_route_id = $2
				WHERE
					id = $1;
				"#,
				&domain_id,
				&cf_route_id
			)
			.execute(&mut *connection)
			.await?;
		}

		log::info!("Completed workspace domain migrations for cf ingress");
		Ok(())
	}

	async fn migrate_managed_url(
		connection: &mut <Database as sqlx::Database>::Connection,
		config: &Settings,
	) -> Result<(), Error> {
		log::info!("Running managed url migrations for cf ingress");

		query!(
			r#"
			ALTER TABLE managed_url
				ADD COLUMN cf_custom_hostname_id TEXT;
			"#
		)
		.execute(&mut *connection)
		.await?;

		// todo: update query based on customhostname issue
		let valid_custom_hostnames = query!(
			r#"
			SELECT
				DISTINCT CONCAT(managed_url.sub_domain, '.', domain.name, '.', domain.tld) as "hostname"
			FROM managed_url
			JOIN workspace_domain
				ON workspace_domain.id = managed_url.domain_id
			JOIN domain
				ON domain.id = workspace_domain.id
			WHERE
				workspace_domain.nameserver_type = 'external' AND
				managed_url.deleted IS NULL;
			"#
		)
		.fetch(&mut *connection)
		.map_ok(|row| {
				row.get::<String, _>("hostname")
		})
		.try_collect::<Vec<_>>()
		.await?;

		for (idx, hostname) in valid_custom_hostnames.iter().enumerate() {
			log::info!(
				" {}/{} - Creating cf custom hostname for {}",
				idx,
				valid_custom_hostnames.len(),
				hostname
			);

			let cf_hostname = hostname.strip_prefix("@.").unwrap_or(hostname);
			let cf_custom_hostname_id =
				create_cf_custom_hostname(cf_hostname, config).await?;

			query!(
				r#"
				UPDATE managed_url
				SET cf_custom_hostname_id = $2
				FROM domain
				WHERE
					managed_url.deleted IS NULL AND
					domain.id = managed_url.domain_id AND
					concat(managed_url.sub_domain, '.', domain.name, '.', domain.tld) = $1;
				"#,
				hostname,
				&cf_custom_hostname_id
			)
			.execute(&mut *connection)
			.await?;
		}

		log::info!("Completed managed url migrations for cf ingress");
		Ok(())
	}

	async fn migrate_byoc_region(
		connection: &mut <Database as sqlx::Database>::Connection,
		_config: &Settings,
	) -> Result<(), Error> {
		log::info!("Running byoc region migrations for cf ingress");

		query!(
			r#"
			ALTER TABLE deployment_region
				ADD COLUMN cf_cert_id TEXT;
			"#
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			ALTER TABLE deployment_region
				DROP CONSTRAINT deployment_region_chk_ready_or_not;
			"#
		)
		.execute(&mut *connection)
		.await?;

		// todo: mark all the running byoc region and their deployments as
		// 		 deleted as currently running ones are used for testing,
		//		 depends on byoc PR

		query!(
			r#"
			ALTER TABLE deployment_region
				ADD CONSTRAINT deployment_region_chk_ready_or_not CHECK(
					(
						workspace_id IS NOT NULL AND
						cf_cert_id IS NOT NULL AND (
							(
								ready = TRUE AND
								kubernetes_cluster_url IS NOT NULL AND
								kubernetes_ca_data IS NOT NULL AND
								kubernetes_auth_username IS NOT NULL AND
								kubernetes_auth_token IS NOT NULL AND
								kubernetes_ingress_ip_addr IS NOT NULL
							) OR (
								ready = FALSE AND
								kubernetes_cluster_url IS NULL AND
								kubernetes_ca_data IS NULL AND
								kubernetes_auth_username IS NULL AND
								kubernetes_auth_username IS NULL AND
								kubernetes_ingress_ip_addr IS NULL
							)
						)
					) OR (
						workspace_id IS NULL AND
						cf_cert_id IS NULL AND
						kubernetes_cluster_url IS NULL AND
						kubernetes_ca_data IS NULL AND
						kubernetes_auth_username IS NULL AND
						kubernetes_auth_token IS NULL AND
						kubernetes_ingress_ip_addr IS NULL
					)
				);
			"#
		)
		.execute(&mut *connection)
		.await?;

		log::info!("Completed byoc region migrations for cf ingress");
		Ok(())
	}

	async fn create_cf_worker_routes_for_domain(
		domain: &str,
		config: &Settings,
	) -> Result<String, Error> {
		let cf_client = CloudflareClient::new(
			Credentials::UserAuthToken {
				token: config.cloudflare.api_token.clone(),
			},
			HttpApiClientConfig::default(),
			Environment::Production,
		)
		.map_err(|err| {
			log::error!("Error while initializing cloudflare client: {}", err);
			Error::empty()
		})?;

		let response = cf_client
			.request_handle(&workers::CreateRoute {
				zone_identifier: &config.cloudflare.patr_zone_identifier,
				params: CreateRouteParams {
					pattern: format!("*{}/*", domain),
					script: Some(config.cloudflare.worker_script.to_owned()),
				},
			})
			.await?;

		Ok(response.result.id)
	}

	async fn create_cf_custom_hostname(
		host: &str,
		config: &Settings,
	) -> Result<String, Error> {
		let cf_client = CloudflareClient::new(
			Credentials::UserAuthToken {
				token: config.cloudflare.api_token.clone(),
			},
			HttpApiClientConfig::default(),
			Environment::Production,
		)
		.map_err(|err| {
			log::error!("Error while initializing cloudflare client: {}", err);
			Error::empty()
		})?;

		let response = cf_client
			.request_handle(&CreateCustomHostname {
				zone_identifier: &config.cloudflare.patr_zone_identifier,
				params: CreateCustomHostnameParams {
					hostname: host.to_owned(),
					ssl: SslParams {
						method: "http".to_owned(),
						type_: "dv".to_owned(),
						settings: SslSettingsParams {
							min_tls_version: "1.0".to_owned(),
							..Default::default()
						},
						..Default::default()
					},
				},
			})
			.await?;

		Ok(response.result.id)
	}
}

pub(super) async fn add_sync_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	log::info!("Adding is_syncing and last_synced column");

	query!(
		r#"
		ALTER TABLE ci_git_provider
		ADD COLUMN is_syncing BOOL NOT NULL DEFAULT FALSE, 
		ADD COLUMN last_synced TIMESTAMPTZ DEFAULT NULL;            
		"#
	)
	.execute(&mut *connection)
	.await?;

	log::info!("Done");

	Ok(())
}
