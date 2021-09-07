use api_macros::{query, query_as};

use crate::{
	models::db_mapping::{
		CloudPlatform,
		Engine,
		ManagedDatabase,
		ManagedDatabaseStatus,
	},
	Database,
};

pub async fn initialize_managed_database_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing managed databases tables");
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
		CREATE TYPE ENGINE AS ENUM(
			'postgres',
			'mysql'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DATABASE_PLAN AS ENUM(
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
			engine ENGINE NOT NULL,
			version TEXT NOT NULL,
			num_nodes INTEGER NOT NULL,
			size DATABASE_PLAN NOT NULL,
			region TEXT NOT NULL,
			status MANAGED_DATABASE_STATUS NOT NULL DEFAULT 'creating',
			host TEXT NOT NULL,
			port INTEGER NOT NULL,
			username TEXT NOT NULL,
			password TEXT NOT NULL,
			organisation_id BYTEA NOT NULL,
			digital_ocean_db_id TEXT
				CONSTRAINT managed_database_uq_digital_ocean_db_id UNIQUE,
			CONSTRAINT managed_database_uq_name_organisation_id
				UNIQUE(name, organisation_id)
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
	log::info!("Finishing up managed databases tables initialization");
	query!(
		r#"
		ALTER TABLE managed_database 
		ADD CONSTRAINT managed_database_repository_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	name: &str,
	db_name: &str,
	engine: &Engine,
	version: &str,
	num_nodes: i32,
	size: DatabasePlan,
	region: &str,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
	organisation_id: &[u8],
	digital_ocean_db_id: Option<&str>,
) -> Result<(), sqlx::Error> {
	if Some(digital_ocean_id) = digital_ocean_db_id {
		query!(
			r#"
			INSERT INTO
				managed_database
			VALUES
				(
					$1,
					$2,
					$3,
					$4,
					$5,
					$6,
					$7,
					$8,
					'creating',
					$9,
					$10,
					$11,
					$12,
					$13,
					$14
				);
			"#,
			id,
			name,
			db_name,
			engine,
			version,
			num_nodes,
			size,
			region,
			host,
			port,
			username,
			password,
			organisation_id,
			digital_ocean_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				managed_database
			VALUES
				(
					$1,
					$2,
					$3,
					$4,
					$5,
					$6,
					$7,
					$8,
					'creating',
					$9,
					$10,
					$11,
					$12,
					$13,
					NULL
				);
			"#,
			id,
			name,
			db_name,
			engine,
			version,
			num_nodes,
			size,
			region,
			host,
			port,
			username,
			password,
			organisation_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn update_managed_database_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_running_database_clusters_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<ManagedDatabase>, sqlx::Error> {
	let rows = query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id,
			name,
			engine as "engine: Engine",
			version,
			num_nodes,
			size,
			region,
			status as "status: _",
			host,
			port,
			username,
			password,
			organisation_id,
			digital_ocean_db_id
		FROM
			managed_database
		WHERE
			organisation_id = $1 AND
			cloud_database_id IS NOT NULL AND
			status = 'running';
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn update_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	cloud_database_id: &str,
	engine: &Engine,
	version: &str,
	num_nodes: i32,
	size: &str,
	region: &str,
	status: ManagedDatabaseStatus,
	host: &str,
	port: i32,
	user: &str,
	password: &str,
	resource_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE 
			managed_database
		SET
			cloud_database_id = $1,
			engine = $2,
			version = $3,
			num_nodes = $4,
			size = $5,
			region = $6,
			status = $7,
			host = $8,
			port = $9,
			username = $10,
			password = $11
		WHERE
			id = $12;
		"#,
		cloud_database_id,
		engine as Engine,
		version,
		num_nodes,
		size,
		region,
		status as ManagedDatabaseStatus,
		host,
		port,
		user,
		password,
		resource_id
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn get_managed_database_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
) -> Result<Option<ManagedDatabase>, sqlx::Error> {
	let row = query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id,
			name,
			cloud_database_id,
			db_provider_name as "db_provider_name: CloudPlatform",
			engine as "engine: Engine",
			version,
			num_nodes,
			size,
			region,
			status as "status: _",
			host,
			port,
			username,
			password,
			organisation_id,
			digital_ocean_db_id
		FROM
			managed_database
		WHERE
			id = $1;
		"#,
		id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}
