use api_models::utils::Uuid;

use crate::{migrate_query as query, utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	make_rbac_descriptions_non_nullable(&mut *connection, config).await?;
	update_roles_permissions(&mut *connection, config).await?;
	add_rbac_user_permissions(&mut *connection, config).await?;

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
				name = #2;
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
		"workspace::delete",
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
