use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	add_tables_for_k8s_database(connection, config).await?;

	Ok(())
}

async fn add_tables_for_k8s_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		DROP TYPE MANAGED_DATABASE_ENGINE CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE managed_database;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE MANAGED_DATABASE_PLAN CASCADE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE managed_database_payment_history;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TYPE MANAGED_DATABASE_STATUS;
		"#
	)
	.execute(&mut *connection)
	.await?;

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
		CREATE TABLE IF NOT EXISTS managed_database_payment_history(
			workspace_id UUID NOT NULL,
			database_id UUID NOT NULL,
			db_plan_id UUID NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
		ADD CONSTRAINT managed_database_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
