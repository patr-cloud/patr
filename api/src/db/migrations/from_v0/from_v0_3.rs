use semver::Version;
use uuid::Uuid;

use crate::{
	db,
	models::{db_mapping::Permission, rbac},
	query,
	query_as,
	Database,
};

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
	// Add region column to deployments
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN region TEXT NOT NULL
		DEFAULT 'do-blr';
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add domain name to deployments
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN domain_name VARCHAR(255)
		CONSTRAINT deployment_uq_domain_name UNIQUE
		CONSTRAINT deployment_chk_domain_name_is_lower_case CHECK(
			name = LOWER(name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add horizontal scale to deployments
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN horizontal_scale SMALLINT NOT NULL
		CONSTRAINT deployment_chk_horizontal_scale_u8 CHECK(
			horizontal_scale >= 0 AND horizontal_scale <= 256
		)
		DEFAULT 1;
		"#
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add deployment machine type to deployments
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN machine_type DEPLOYMENT_MACHINE_TYPE NOT NULL
		DEFAULT 'small';
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add environment variables type for deployments
	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id BYTEA
				CONSTRAINT deploymment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT NOT NULL,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Insert new permissions into the database for managed databases
	for &permission in [
		rbac::permissions::organisation::managed_database::CREATE,
		rbac::permissions::organisation::managed_database::LIST,
		rbac::permissions::organisation::managed_database::DELETE,
		rbac::permissions::organisation::managed_database::INFO,
	].iter() {
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query_as!(
				Permission,
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
			.fetch_all(&mut *connection)
			.await?
			.into_iter()
			.next()
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
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	// Insert new resource type into the database for managed database
	let (resource_type, uuid) = (
		rbac::resource_types::MANAGED_DATABASE.to_string(),
		db::generate_new_resource_type_id(&mut *connection)
			.await?
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
	db::initialize_managed_database_pre(connection).await?;
	db::initialize_managed_database_post(connection).await?;

	Ok(())
}
