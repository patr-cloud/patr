use api_macros::{query, query_as};

use crate::{
	models::db_mapping::{
		ManagedDatabase,
		ManagedDatabaseEngine,
		ManagedDatabasePlan,
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
			name CITEXT NOT NULL
				CONSTRAINT managed_database_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			db_name VARCHAR(255) NOT NULL
				CONSTRAINT managed_database_chk_db_name_is_trimmed CHECK(
					db_name = TRIM(db_name)
				),
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
			workspace_id BYTEA NOT NULL,
			digitalocean_db_id TEXT
				CONSTRAINT managed_database_uq_digitalocean_db_id UNIQUE,
			CONSTRAINT managed_database_uq_name_workspace_id
				UNIQUE(name, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_managed_database_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up managed databases tables initialization");
	query!(
		r#"
		ALTER TABLE managed_database 
		ADD CONSTRAINT managed_database_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
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
	engine: &ManagedDatabaseEngine,
	version: &str,
	num_nodes: u64,
	database_plan: &ManagedDatabasePlan,
	region: &str,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
	workspace_id: &[u8],
	digital_ocean_id: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(digitalocean_db_id) = digital_ocean_id {
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
			name as _,
			db_name,
			engine as _,
			version,
			num_nodes as i32,
			database_plan as _,
			region,
			host,
			port,
			username,
			password,
			workspace_id,
			digitalocean_db_id
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
			name as _,
			db_name,
			engine as _,
			version,
			num_nodes as i32,
			database_plan as _,
			region,
			host,
			port,
			username,
			password,
			workspace_id,
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

pub async fn update_managed_database_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name,
		database_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_database_clusters_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
) -> Result<Vec<ManagedDatabase>, sqlx::Error> {
	query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id,
			name as "name: _",
			db_name,
			engine as "engine: _",
			version,
			num_nodes,
			database_plan as "database_plan: _",
			region,
			status as "status: _",
			host,
			port,
			username,
			password,
			workspace_id,
			digitalocean_db_id
		FROM
			managed_database
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id
	)
	.fetch_all(&mut *connection)
	.await
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
			name as "name: _",
			db_name,
			engine as "engine: _",
			version,
			num_nodes,
			database_plan as "database_plan: _",
			region,
			status as "status: _",
			host,
			port,
			username,
			password,
			workspace_id,
			digitalocean_db_id
		FROM
			managed_database
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn update_digitalocean_db_id_for_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	digitalocean_db_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
		SET
			digitalocean_db_id = $1
		WHERE
			id = $2;
		"#,
		digitalocean_db_id,
		database_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_database_credentials_for_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &[u8],
	host: &str,
	port: i32,
	username: &str,
	password: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
		SET
			host = $1,
			port = $2,
			username = $3,
			password = $4
		WHERE
			id = $5;
		"#,
		host,
		port,
		username,
		password,
		database_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
