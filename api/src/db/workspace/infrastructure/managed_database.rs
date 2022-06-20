use std::{fmt::Display, str::FromStr};

use api_macros::{query, query_as};
use api_models::utils::Uuid;
use eve_rs::AsError;

use crate::{error, utils::Error, Database};

#[derive(sqlx::Type)]
#[sqlx(type_name = "MANAGED_DATABASE_STATUS", rename_all = "lowercase")]
pub enum ManagedDatabaseStatus {
	Creating,
	Running,
	Errored,
	Deleted,
}

impl Display for ManagedDatabaseStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Creating => write!(f, "creating"),
			Self::Running => write!(f, "running"),
			Self::Errored => write!(f, "errored"),
			Self::Deleted => write!(f, "deleted"),
		}
	}
}

impl FromStr for ManagedDatabaseStatus {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"configuring-log-exports" |
			"started" |
			"creating" |
			"backing-up" |
			"notstarted" => Ok(Self::Creating),
			"running" | "online" | "created" | "completed" | "succeeded" |
			"available" => Ok(Self::Running),
			"errored" | "failed" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "MANAGED_DATABASE_ENGINE", rename_all = "lowercase")]
pub enum ManagedDatabaseEngine {
	Postgres,
	Mysql,
}

impl Display for ManagedDatabaseEngine {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Postgres => write!(f, "postgres"),
			Self::Mysql => write!(f, "mysql"),
		}
	}
}

impl FromStr for ManagedDatabaseEngine {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"pg" | "postgres" | "postgresql" => Ok(Self::Postgres),
			"mysql" => Ok(Self::Mysql),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "MANAGED_DATABASE_PLAN", rename_all = "lowercase")]
pub enum ManagedDatabasePlan {
	Nano,
	Micro,
	Small,
	Medium,
	Large,
	Xlarge,
	Xxlarge,
	Mammoth,
}

impl Display for ManagedDatabasePlan {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Nano => write!(f, "nano"),
			Self::Micro => write!(f, "micro"),
			Self::Small => write!(f, "small"),
			Self::Medium => write!(f, "medium"),
			Self::Large => write!(f, "large"),
			Self::Xlarge => write!(f, "xlarge"),
			Self::Xxlarge => write!(f, "xxlarge"),
			Self::Mammoth => write!(f, "mammoth"),
		}
	}
}

impl FromStr for ManagedDatabasePlan {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"do-nano" | "db-s-1vcpu-1gb" => Ok(Self::Nano),
			"do-micro" | "db-s-1vcpu-2gb" | "aws-micro" | "micro_1_0" => {
				Ok(Self::Micro)
			}
			"aws-small" | "small_1_0" => Ok(Self::Small),
			"do-medium" | "db-s-2vcpu-4gb" | "aws-medium" | "medium_1_0" => {
				Ok(Self::Medium)
			}
			"do-large" | "db-s-4vcpu-8gb" | "aws-large" | "large_1_0" => {
				Ok(Self::Large)
			}
			"do-xlarge" | "db-s-6vcpu-16gb" => Ok(Self::Xlarge),
			"do-xxlarge" | "db-s-8vcpu-32gb" => Ok(Self::Xxlarge),
			"do-mammoth" | "db-s-16vcpu-64gb" => Ok(Self::Mammoth),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

pub struct ManagedDatabase {
	pub id: Uuid,
	pub name: String,
	pub db_name: String,
	pub engine: ManagedDatabaseEngine,
	pub version: String,
	pub num_nodes: i32,
	pub database_plan: ManagedDatabasePlan,
	pub region: String,
	pub status: ManagedDatabaseStatus,
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub workspace_id: Uuid,
	pub digitalocean_db_id: Option<String>,
	pub database_payment_history_id: Option<Uuid>,
}

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
			id UUID CONSTRAINT managed_database_pk PRIMARY KEY,
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
			workspace_id UUID NOT NULL,
			digitalocean_db_id TEXT
				CONSTRAINT managed_database_uq_digitalocean_db_id UNIQUE,
			database_payment_history_id UUID,
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

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_fk_database_payment_history_id
		FOREIGN KEY(database_payment_history_id) REFERENCES managed_database_payment_history(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
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
	workspace_id: &Uuid,
	digital_ocean_id: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(digitalocean_db_id) = digital_ocean_id {
		query!(
			r#"
			INSERT INTO
				managed_database(
					id,
					name,
					db_name,
					engine,
					version,
					num_nodes,
					database_plan,
					region,
					status,
					host,
					port,
					username,
					password,
					workspace_id,
					digitalocean_db_id
				)
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
			id as _,
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
			workspace_id as _,
			digitalocean_db_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				managed_database(
					id,
					name,
					db_name,
					engine,
					version,
					num_nodes,
					database_plan,
					region,
					status,
					host,
					port,
					username,
					password,
					workspace_id,
					digitalocean_db_id
				)
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
			id as _,
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
			workspace_id as _,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn update_managed_database_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
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
		id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_database_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
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
		name as _,
		database_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_database_clusters_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<ManagedDatabase>, sqlx::Error> {
	query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
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
			workspace_id as "workspace_id: _",
			digitalocean_db_id,
			database_payment_history_id as "database_payment_history_id: _"
		FROM
			managed_database
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_managed_database_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<ManagedDatabase>, sqlx::Error> {
	query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
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
			workspace_id as "workspace_id: _",
			digitalocean_db_id,
			database_payment_history_id as "database_payment_history_id: _"
		FROM
			managed_database
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_digitalocean_db_id_for_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
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
		database_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_database_credentials_for_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
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
		database_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
