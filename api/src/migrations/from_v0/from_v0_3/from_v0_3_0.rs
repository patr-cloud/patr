use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	models::rbac,
	utils::settings::Settings,
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	// Rename the unqiue constraint from uk to uq
	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_uk_username
		TO user_uq_username;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_uk_bckp_eml_lcl_bckp_eml_dmn_id
		TO user_uq_backup_email_local_backup_email_domain_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_uk_bckp_phn_cntry_cd_bckp_phn_nmbr
		TO user_uq_backup_phone_country_code_backup_phone_number;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Add deployment machine type enum
	query!(
		r#"
		CREATE TYPE DEPLOYMENT_MACHINE_TYPE AS ENUM(
			'micro',
			'small',
			'medium',
			'large'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Add region, domain name, horizontal scale, machine type and
	// organisation_id columns to deployment table
	query!(
		r#"
		ALTER TABLE deployment
			ADD COLUMN region TEXT NOT NULL
				DEFAULT 'do-blr',
			ADD COLUMN domain_name VARCHAR(255)
				CONSTRAINT deployment_uq_domain_name UNIQUE
				CONSTRAINT deployment_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			ADD COLUMN horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_horizontal_scale_u8 CHECK(
					horizontal_scale >= 0 AND horizontal_scale <= 256
				)
				DEFAULT 1,
			ADD COLUMN machine_type DEPLOYMENT_MACHINE_TYPE NOT NULL
				DEFAULT 'small',
			ADD COLUMN organisation_id BYTEA;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Change digital_ocean_app_id for deployment to digitalocean_app_id
	query!(
		r#"
		ALTER TABLE deployment
		RENAME COLUMN digital_ocean_app_id TO digitalocean_app_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Rename the unqiue constraint for digitalocean_app_id
	query!(
		r#"
		ALTER TABLE deployment
		RENAME CONSTRAINT deployment_uq_digital_ocean_app_id
		TO deployment_uq_digitalocean_app_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Set the organisation_id based on the resource.owner_id
	query!(
		r#"
		UPDATE
			deployment
		SET
			organisation_id = resource.owner_id
		FROM
			resource
		WHERE
			resource.id = deployment.id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Set organisation_id to not null
	// Remove the deployment.id -> resource.id foreign key
	// Add a deployment(id, organisation_id) -> resource(id, owner_id) foreign
	// key Add a deployment(name, organisation_id) unique constraint
	query!(
		r#"
		ALTER TABLE deployment
			ALTER COLUMN organisation_id
				SET NOT NULL,
			DROP CONSTRAINT deployment_fk_id,
			ADD CONSTRAINT deployment_fk_id_organisation_id
				FOREIGN KEY(id, organisation_id)
					REFERENCES resource(id, owner_id),
			ADD CONSTRAINT deployment_uq_name_organisation_id
				UNIQUE(name, organisation_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Add environment variables type for deployments
	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id BYTEA
				CONSTRAINT deployment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT NOT NULL,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Insert new permissions into the database for managed databases
	for &permission in [
		"organisation::managedDatabase::create",
		"organisation::managedDatabase::list",
		"organisation::managedDatabase::delete",
		"organisation::managedDatabase::info",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
				WHERE
					id = $1;
				"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		let uuid = uuid.as_bytes().as_ref();
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, NULL);
			"#,
			uuid,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	// Insert new resource type into the database for managed database
	let (resource_type, uuid) = (
		rbac::resource_types::MANAGED_DATABASE.to_string(),
		loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		}
		.as_bytes()
		.to_vec(),
	);
	query!(
		r#"
		INSERT INTO
			resource_type
		VALUES
			($1, $2, NULL);
		"#,
		uuid,
		resource_type,
	)
	.execute(&mut *connection)
	.await?;

	// Create tables for managed databases
	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_STATUS AS ENUM(
			'creating', /* Started the creation of database */
			'running', /* Database is running successfully */
			'stopped', /* Database is stopped by the user */
			'errored', /* Database encountered errors */
			'deleted' /* Database is deled by the user   */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_ENGINE AS ENUM(
			'postgres',
			'mysql'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_PLAN AS ENUM(
			'nano',
			'micro',
			'small',
			'medium',
			'large',
			'xlarge',
			'xxlarge',
			'mammoth'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database(
			id BYTEA CONSTRAINT managed_database_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			db_name VARCHAR(255) NOT NULL,
			engine MANAGED_DATABASE_ENGINE NOT NULL,
			version TEXT NOT NULL,
			num_nodes INTEGER NOT NULL,
			database_plan MANAGED_DATABASE_PLAN NOT NULL,
			region TEXT NOT NULL,
			status MANAGED_DATABASE_STATUS NOT NULL,
			host TEXT NOT NULL,
			port INTEGER NOT NULL,
			username TEXT NOT NULL,
			password TEXT NOT NULL,
			organisation_id BYTEA NOT NULL,
			digitalocean_db_id TEXT
				CONSTRAINT managed_database_uq_digitalocean_db_id UNIQUE,
			CONSTRAINT managed_database_uq_name_organisation_id
				UNIQUE(name, organisation_id),
			CONSTRAINT managed_database_repository_fk_id_organisation_id
				FOREIGN KEY(id, organisation_id) REFERENCES
					resource(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_REQUEST_PROTOCOL AS ENUM(
			'http',
			'https'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_REQUEST_METHOD AS ENUM(
			'get',
			'post',
			'put',
			'delete',
			'head',
			'options',
			'connect',
			'patch'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_request_logs(
			id BIGSERIAL PRIMARY KEY,
			deployment_id BYTEA NOT NULL
				CONSTRAINT deployment_request_logs_fk_deployment_id
					REFERENCES deployment(id),
			timestamp BIGINT NOT NULL
				CONSTRAINT deployment_request_logs_chk_unsigned
						CHECK(timestamp >= 0),
			ip_address VARCHAR(255) NOT NULL,
			ip_address_location GEOMETRY NOT NULL,
			method DEPLOYMENT_REQUEST_METHOD NOT NULL,
			host VARCHAR(255) NOT NULL
				CONSTRAINT deployment_request_logs_chk_host_is_lower_case
					CHECK(host = LOWER(host)),
			protocol DEPLOYMENT_REQUEST_PROTOCOL NOT NULL,
			path TEXT NOT NULL,
			response_time REAL NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE data_center_locations(
			region TEXT CONSTRAINT data_center_locations_pk PRIMARY KEY,
			location GEOMETRY NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			data_center_locations
		VALUES
			('aws-us-east-1', ST_SetSRID(POINT(-77.4524237, 38.9940541)::GEOMETRY, 4326)),
			('aws-us-east-2', ST_SetSRID(POINT(-82.7541337, 40.0946354)::GEOMETRY, 4326)),
			('aws-us-west-2', ST_SetSRID(POINT(-119.2684488, 45.9174667)::GEOMETRY, 4326)),
			('aws-eu-west-1', ST_SetSRID(POINT(-6.224503, 53.4056545)::GEOMETRY, 4326)),
			('aws-eu-west-2', ST_SetSRID(POINT(-0.0609266, 51.5085036)::GEOMETRY, 4326)),
			('aws-eu-west-3', ST_SetSRID(POINT(2.2976644, 48.6009709)::GEOMETRY, 4326)),
			('aws-eu-central-1', ST_SetSRID(POINT(8.6303932, 50.0992094)::GEOMETRY, 4326)),
			('aws-ap-southeast-1', ST_SetSRID(POINT(103.6930643, 1.3218269)::GEOMETRY, 4326)),
			('aws-ap-southeast-2', ST_SetSRID(POINT(151.1907535, -33.9117717)::GEOMETRY, 4326)),
			('aws-ap-northeast-1', ST_SetSRID(POINT(139.7459176, 35.617436)::GEOMETRY, 4326)),
			('aws-ap-northeast-2', ST_SetSRID(POINT(126.8736237, 37.5616592)::GEOMETRY, 4326)),
			('aws-ap-south-1', ST_SetSRID(POINT(72.9667878, 19.2425503)::GEOMETRY, 4326)),
			('aws-ca-central-1', ST_SetSRID(POINT(-73.6, 45.5)::GEOMETRY, 4326)),
			('aws-eu-north-1', ST_SetSRID(POINT(17.8419717, 59.326242)::GEOMETRY, 4326)),
			('do-tor', ST_SetSRID(POINT(-79.3623, 43.6547)::GEOMETRY, 4326)),
			('do-sfo', ST_SetSRID(POINT(-121.9753, 37.3417)::GEOMETRY, 4326)),
			('do-nyc', ST_SetSRID(POINT(-73.981, 40.7597)::GEOMETRY, 4326)),
			('do-lon', ST_SetSRID(POINT(-0.6289, 51.5225)::GEOMETRY, 4326)),
			('do-ams', ST_SetSRID(POINT(4.9479, 52.3006)::GEOMETRY, 4326)),
			('do-sgp', ST_SetSRID(POINT(103.695, 1.32123)::GEOMETRY, 4326)),
			('do-fra', ST_SetSRID(POINT(8.6843, 50.1188)::GEOMETRY, 4326)),
			('do-blr', ST_SetSRID(POINT(77.5855, 12.9634)::GEOMETRY, 4326));
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Insert new permissions into the database for static sites
	for &permission in [
		"organisation::staticSite::list",
		"organisation::staticSite::create",
		"organisation::staticSite::info",
		"organisation::staticSite::delete",
		"organisation::staticSite::edit",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
					SELECT
						*
					FROM
						permission
					WHERE
						id = $1;
					"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		let uuid = uuid.as_bytes().as_ref();
		query!(
			r#"
				INSERT INTO
					permission
				VALUES
					($1, $2, NULL);
				"#,
			uuid,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	// Insert new resource type into the database for managed database
	let (resource_type, uuid) = (
		rbac::resource_types::STATIC_SITE.to_string(),
		loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
					SELECT
						*
					FROM
						resource_type
					WHERE
						id = $1;
					"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		}
		.as_bytes()
		.to_vec(),
	);

	query!(
		r#"
			INSERT INTO
				resource_type
			VALUES
				($1, $2, NULL);
			"#,
		uuid,
		resource_type,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_static_sites(
			id BYTEA CONSTRAINT deployment_static_sites_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			domain_name VARCHAR(255)
				CONSTRAINT deployment_static_sites_pk_uq_domain_name UNIQUE
				CONSTRAINT
					deployment_static_sites_pk_chk_domain_name_is_lower_case
						CHECK(domain_name = LOWER(domain_name)),
			organisation_id BYTEA NOT NULL,
			CONSTRAINT deployment_static_sites_uq_name_organisation_id
				UNIQUE(name, organisation_id),
			CONSTRAINT deployment_static_sites_fk_id_organisation_id
				FOREIGN KEY(id, organisation_id) REFERENCES
					resource(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
