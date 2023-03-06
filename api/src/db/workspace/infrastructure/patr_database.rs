use api_macros::{query, query_as};
use api_models::{
	models::workspace::infrastructure::database::{
		PatrDatabaseEngine,
		PatrDatabasePlan,
		PatrDatabaseStatus,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::Database;

pub struct PatrDatabase {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
	pub region: Uuid,
	pub db_name: String,
	pub engine: PatrDatabaseEngine,
	pub version: String,
	pub database_plan: PatrDatabasePlan,
	pub status: PatrDatabaseStatus,
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub replica_numbers: i32,
}

pub async fn initialize_patr_database_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing patr databases tables");

	query!(
		r#"
		CREATE TYPE PATR_DATABASE_ENGINE AS ENUM(
			'postgres',
			'mysql',
			'redis'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// 1r == 1GB ram
	// 1c == 1v cpu
	// 1v == 1GB volume
	query!(
		r#"
		CREATE TYPE PATR_DATABASE_PLAN AS ENUM(
			'db_1r_1c_10v',
			'db_2r_2c_25v'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PATR_DATABASE_STATUS AS ENUM(
			'creating',
			'running',
			'errored',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_database(
			id 				UUID					NOT NULL,
			name 			CITEXT 					NOT NULL,
			workspace_id 	UUID 					NOT NULL,
			region 			UUID 					NOT NULL,
			db_name 		VARCHAR(255) 			NOT NULL,
			engine 			PATR_DATABASE_ENGINE 	NOT NULL,
			version 		TEXT 					NOT NULL,
			database_plan 	PATR_DATABASE_PLAN 		NOT NULL,
			status 			PATR_DATABASE_STATUS 	NOT NULL DEFAULT 'creating',
			host 			TEXT 					NOT NULL,
			port 			INTEGER 				NOT NULL,
			username 		TEXT 					NOT NULL,
			password 		TEXT 					NOT NULL,
			replica_numbers INTEGER					NOT NULL DEFAULT 1,
			deleted 		TIMESTAMPTZ,

			CONSTRAINT patr_database_pk
				PRIMARY KEY (id),
		
			CONSTRAINT patr_database_chk_name_is_trimmed
				CHECK(name = TRIM(name)),
			CONSTRAINT patr_database_chk_db_name_is_trimmed
				CHECK(db_name = TRIM(db_name)),
		
			CONSTRAINT patr_database_fk_region
				FOREIGN KEY (region) REFERENCES deployment_region(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			patr_database_uq_workspace_id_name
		ON
			patr_database(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_patr_database_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up patr databases tables initialization");
	query!(
		r#"
		ALTER TABLE patr_database
			ADD CONSTRAINT patr_database_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_patr_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	db_name: &str,
	engine: &PatrDatabaseEngine,
	version: &str,
	database_plan: &PatrDatabasePlan,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
	replica_numbers: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO patr_database (
			id,
			name,
			workspace_id,
			region,
			db_name,
			engine,
			version,
			database_plan,
			host,
			port,
			username,
			password,
			replica_numbers
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13);
		"#,
		id as _,
		name as _,
		workspace_id as _,
		region as _,
		db_name,
		engine as _,
		version,
		database_plan as _,
		host,
		port,
		username,
		password,
		replica_numbers,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_patr_database_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	status: &PatrDatabaseStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			patr_database
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_patr_database_replicas(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	replica_numbers: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			patr_database
		SET
			replica_numbers = $1
		WHERE
			id = $2;
		"#,
		replica_numbers as _,
		id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_patr_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			patr_database
		SET
			deleted = $2,
			status = 'deleted'
		WHERE
			id = $1;
		"#,
		database_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_patr_database_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<PatrDatabase>, sqlx::Error> {
	query_as!(
		PatrDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			db_name,
			engine as "engine: _",
			version,
			database_plan as "database_plan: _",
			status as "status: _",
			host,
			port,
			username,
			password,
			replica_numbers
		FROM
			patr_database
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_patr_database_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<PatrDatabase>, sqlx::Error> {
	query_as!(
		PatrDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			db_name,
			engine as "engine: _",
			version,
			database_plan as "database_plan: _",
			status as "status: _",
			host,
			port,
			username,
			password,
			replica_numbers
		FROM
			patr_database
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_patr_database_by_id_including_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<PatrDatabase>, sqlx::Error> {
	query_as!(
		PatrDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			db_name,
			engine as "engine: _",
			version,
			database_plan as "database_plan: _",
			status as "status: _",
			host,
			port,
			username,
			password,
			replica_numbers
		FROM
			patr_database
		WHERE
			id = $1;
		"#,
		id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}
