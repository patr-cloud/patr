use crate::prelude::*;

/// Initializes the managed database tables
#[instrument(skip(connection))]
pub async fn initialize_managed_database_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {

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
		CREATE TYPE LEGACY_MANAGED_DATABASE_PLAN AS ENUM(
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
		CREATE TABLE managed_database_plan(
			id 		UUID NOT NULL,
			cpu 	INTEGER NOT NULL, /* Multiples of 1 vCPU */
			ram 	INTEGER NOT NULL, /* Multiples of 1 GB RAM */
			volume 	INTEGER NOT NULL /* Multiples of 1 GB storage space */
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
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			workspace_id UUID NOT NULL,
			region UUID NOT NULL,
			engine MANAGED_DATABASE_ENGINE NOT NULL,
			database_plan_id UUID NOT NULL,
			status MANAGED_DATABASE_STATUS NOT NULL,
			username TEXT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the managed database constraints
#[instrument(skip(connection))]
pub async fn initialize_managed_database_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {

	query!(
		r#"
		ALTER TABLE managed_database_plan
			ADD CONSTRAINT managed_database_plan_pk PRIMARY KEY(id),
			ADD CONSTRAINT managed_database_plan_chk_cpu_positive CHECK(cpu > 0),
			ADD CONSTRAINT managed_database_plan_chk_ram_positive CHECK(ram > 0),
			ADD CONSTRAINT managed_database_plan_chk_volume_positive
				CHECK(volume > 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
			ADD CONSTRAINT managed_database_pk PRIMARY KEY(id),
			ADD CONSTRAINT managed_database_chk_name_is_trimmed CHECK(
				name = TRIM(name)
			),
			ADD CONSTRAINT managed_database_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT managed_database_fk_region
				FOREIGN KEY(region) REFERENCES region(id),
			ADD CONSTRAINT managed_database_fk_managed_database_plan_id
				FOREIGN KEY(database_plan_id)
					REFERENCES managed_database_plan(id),
			ADD CONSTRAINT managed_database_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
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