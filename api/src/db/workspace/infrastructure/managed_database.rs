use api_macros::{query, query_as};
use api_models::{
	models::workspace::infrastructure::database::{
		DatabasePlanType,
		ManagedDatabaseEngine,
		ManagedDatabaseStatus,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::{db, Database};

pub struct ManagedDatabase {
	pub id: Uuid,
	pub name: String,
	pub workspace_id: Uuid,
	pub region: Uuid,
	pub engine: ManagedDatabaseEngine,
	pub database_plan_id: Uuid,
	pub status: ManagedDatabaseStatus,
	pub username: String,
}

pub async fn initialize_managed_database_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing patr databases tables");

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_ENGINE AS ENUM(
			'postgres',
			'mysql',
			'mongo',
			'redis'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database_plan(
			id 		UUID CONSTRAINT managed_database_plan_pk PRIMARY KEY,
			cpu 	INTEGER NOT NULL, /* Multiples of 1 vCPU */
			ram 	INTEGER NOT NULL, /* Multiples of 1 GB RAM */
			volume 	INTEGER NOT NULL, /* Multiples of 1 GB storage space */

			CONSTRAINT managed_database_plan_chk_cpu_positive CHECK(cpu > 0),
			CONSTRAINT managed_database_plan_chk_ram_positive CHECK(ram > 0),
			CONSTRAINT managed_database_plan_chk_volume_positive
				CHECK(volume > 0)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_STATUS AS ENUM(
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
		CREATE TABLE managed_database(
			id 					UUID						NOT NULL,
			name 				CITEXT 						NOT NULL,
			workspace_id 		UUID 						NOT NULL,
			region 				UUID 						NOT NULL,
			engine 				MANAGED_DATABASE_ENGINE 	NOT NULL,
			database_plan_id 	UUID 						NOT NULL,
			status 				MANAGED_DATABASE_STATUS 	NOT NULL,
			username 			TEXT 						NOT NULL,
			deleted 			TIMESTAMPTZ,

			CONSTRAINT managed_database_pk PRIMARY KEY(id),
			CONSTRAINT managed_database_chk_name_is_trimmed CHECK(
				name = TRIM(name)
			),
			CONSTRAINT managed_database_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			CONSTRAINT managed_database_fk_region
				FOREIGN KEY(region) REFERENCES region(id),
			CONSTRAINT managed_database_fk_managed_database_plan_id
				FOREIGN KEY(database_plan_id)
					REFERENCES managed_database_plan(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_database_uq_workspace_id_name
		ON
			managed_database(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_managed_database_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up patr databases tables initialization");

	const MANAGED_DATABASE_MACHINE_TYPE: [(i32, i32, i32); 2] = [
		(1, 1, 10), // 1 vCPU, 1 GB RAM, 10GB storage
		(2, 2, 25), // 2 vCPU, 2 GB RAM, 25GB storage
	];

	for (cpu, ram, volume) in MANAGED_DATABASE_MACHINE_TYPE {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				managed_database_plan(
					id,
					cpu,
					ram,
					volume
				)
			VALUES
				($1, $2, $3, $4);
			"#,
			machine_type_id as _,
			cpu,
			ram,
			volume
		)
		.execute(&mut *connection)
		.await?;
	}

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
	id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	engine: &ManagedDatabaseEngine,
	database_plan_id: &Uuid,
	username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_database(
				id,
				name,
				workspace_id,
				region,
				status,
				engine,
				database_plan_id,
				username
			)
		VALUES
			($1, $2, $3, $4, 'creating', $5, $6, $7);
		"#,
		id as _,
		name as _,
		workspace_id as _,
		region as _,
		engine as _,
		database_plan_id as _,
		username,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
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

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
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

pub async fn get_all_managed_database_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<ManagedDatabase>, sqlx::Error> {
	query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			engine as "engine: _",
			database_plan_id as "database_plan_id: _",
			status as "status: _",
			username
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
			workspace_id as "workspace_id: _",
			region as "region: _",
			engine as "engine: _",
			database_plan_id as "database_plan_id: _",
			status as "status: _",
			username
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

pub async fn get_managed_database_by_id_including_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<ManagedDatabase>, sqlx::Error> {
	query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			engine as "engine: _",
			database_plan_id as "database_plan_id: _",
			status as "status: _",
			username
		FROM
			managed_database
		WHERE
			id = $1;
		"#,
		id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_database_plans(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<DatabasePlanType>, sqlx::Error> {
	query_as!(
		DatabasePlanType,
		r#"
		SELECT
			id as "id: _",
			cpu as "cpu_count: _",
			ram as "memory_count: _",
			volume
		FROM
			managed_database_plan;
		"#,
	)
	.fetch_all(connection)
	.await
}

pub async fn get_database_plan_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<DatabasePlanType, sqlx::Error> {
	query_as!(
		DatabasePlanType,
		r#"
		SELECT
			id as "id: _",
			cpu as "cpu_count: _",
			ram as "memory_count: _",
			volume
		FROM
			managed_database_plan
		WHERE
			id = $1;
		"#,
		id as _,
	)
	.fetch_one(&mut *connection)
	.await
}
