use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_is_frozen_column_to_workspace(connection, config).await?;
	add_survey_table(connection, config).await?;

	rbac_related_migrations(connection, config).await?;

	Ok(())
}

async fn add_is_frozen_column_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN is_frozen BOOLEAN NOT NULL DEFAULT FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN is_frozen DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_survey_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE SURVEY(
			user_id UUID NOT NULL
				CONSTRAINT survey_fk_user_id REFERENCES "user"(id),
			version TEXT,
			response JSON NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}


async fn rbac_related_migrations(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	rename_role_permissions_resource_tables(&mut *connection, config).await?;
	create_role_block_permissions_resource_table(&mut *connection, config)
		.await?;

	Ok(())
}

async fn rename_role_permissions_resource_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE
			role_permissions_resource
		RENAME TO
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_permissions_resource_type
		RENAME TO
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_role_id
		TO
			role_allow_permissions_resource_type_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_permission_id
		TO
			role_allow_permissions_resource_type_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_resource_type_id
		TO
			role_allow_permissions_resource_type_fk_resource_type_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_pk
		TO
			role_allow_permissions_resource_type_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_role_id
		TO
			role_allow_permissions_resource_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_permission_id
		TO
			role_allow_permissions_resource_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_resource_id
		TO
			role_allow_permissions_resource_fk_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_pk
		TO
			role_allow_permissions_resource_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id
		RENAME TO
			role_allow_permissions_resource_type_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id_resource_type_id
		RENAME TO
			role_allow_permissions_resource_type_idx_roleid_resourcetypeid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id_resource_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_role_block_permissions_resource_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE role_block_permissions_resource(
			role_id UUID
				CONSTRAINT role_block_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_block_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_block_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_block_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id
		ON
			role_block_permissions_resource(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id_resource_id
		ON
			role_block_permissions_resource(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
