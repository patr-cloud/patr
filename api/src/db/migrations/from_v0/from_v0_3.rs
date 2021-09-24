use semver::Version;
use uuid::Uuid;

use crate::{migrate_query as query, models::rbac, Database};

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.x.x, more info here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: Version,
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 3, 0) => migrate_from_v0_3_0(&mut *connection).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

/// # Description
/// The function is used to get a list of all 0.3.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.3.0"]
}

async fn migrate_from_v0_3_0(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
		rbac::permissions::organisation::managed_database::CREATE,
		rbac::permissions::organisation::managed_database::LIST,
		rbac::permissions::organisation::managed_database::DELETE,
		rbac::permissions::organisation::managed_database::INFO,
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
				UNIQUE(name, organisation_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database 
		ADD CONSTRAINT managed_database_repository_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE EXTENSION IF NOT EXISTS postgis;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PROTOCOL AS ENUM(
			'http',
			'https'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE METHOD AS ENUM(
			'post',
			'get',
			'put',
			'patch',
			'delete'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_request_logs(
			id BYTEA CONSTRAINT deployment_request_logs_pk PRIMARY KEY,
			deployment_id BYTEA NOT NULL
				CONSTRAINT deploymment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			ip_address VARCHAR(255) NOT NULL,
			ip_address_location POINT NOT NULL,
			method METHOD NOT NULL,
			domain_name VARCHAR(255) NOT NULL
				CONSTRAINT deployment_request_logs_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			protocol PROTOCOL NOT NULL,
			path TEXT NOT NULL,
			response_time REAL NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_request_logs
		ADD CONSTRAINT deployment_request_logs_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
