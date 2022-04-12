use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	models::rbac,
	utils::settings::Settings,
	Database,
};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	make_rbac_descriptions_non_nullable(&mut *connection, config).await?;
	update_roles_permissions(&mut *connection, config).await?;
	add_rbac_user_permissions(&mut *connection, config).await?;
	update_edit_workspace_permission(&mut *connection, config).await?;
	add_delete_workspace_permission(&mut *connection, config).await?;
	migrate_to_secret(&mut *connection, config).await;
	add_secret_id_column_to_deployment_environment_variable(
		&mut *connection,
		config,
	)
	.await;

	Ok(())
}

async fn make_rbac_descriptions_non_nullable(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE resource_type
		SET description = $1;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource_type
		ALTER COLUMN description SET NOT NULL;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE role
		SET description = $1;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role
		ALTER COLUMN description SET NOT NULL;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE permission
		SET description = $1;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE permission
		ALTER COLUMN description SET NOT NULL;
		"#,
		""
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_roles_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	for (old_permission, new_permission) in [
		("workspace::viewRoles", "workspace::rbac::role::list"),
		("workspace::createRole", "workspace::rbac::role::create"),
		("workspace::editRole", "workspace::rbac::role::edit"),
		("workspace::deleteRole", "workspace::rbac::role::delete"),
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = $2;
			"#,
			new_permission,
			old_permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn add_rbac_user_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	for &permission in [
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
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
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();
			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, $3);
			"#,
			&uuid,
			permission,
			"",
		)
		.execute(&mut *connection)
		.await?;
	}
	Ok(())
}
async fn update_edit_workspace_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			permission
		SET
			name = $1
		WHERE
			name = $2;
		"#,
		"workspace::edit",
		"workspace::editInfo",
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn add_delete_workspace_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
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
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			// That particular resource ID doesn't exist. Use it
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			permission
		VALUES
			($1, $2, $3);
		"#,
		&uuid,
		"workspace::delete",
		"",
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_to_secret(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE secret(
			id UUID CONSTRAINT secret_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			workspace_id UUID NOT NULL,
			deployment_id UUID, /* For deployment specific secrets */
			CONSTRAINT secret_uq_workspace_id_name UNIQUE(workspace_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for &permission in [
		"workspace::infrastructure::secret::create",
		"workspace::infrastructure::secret::delete",
		"workspace::infrastructure::secret::info",
		"workspace::infrastructure::secret::list",
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

	// Insert new resource type into the database for secrets
	let (resource_type, uuid) = (
		rbac::resource_types::SECRET.to_string(),
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
		INSERT INTO,
			resource_type
		VALUES
			($1, $2, NULL);
		"#,
		uuid,
		resource_type,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_secret_id_column_to_deployment_environment_variable(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD COLUMN secret_id UUID;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
			ALTER COLUMN value DROP NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
			ADD CONSTRAINT deployment_environment_variable_fk_secret_id
				FOREIGN KEY(secret_id) REFERENCES secret(id),
			ADD CONSTRAINT deployment_environment_variable_chk_value_secret_id_both_not_null
				CHECK((value IS NULL OR secret_id IS NULL) 
				AND NOT (value IS NULL AND secret_id IS NULL));
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
