use std::collections::HashMap;

use api_models::utils::Uuid;

use crate::{query, query_as, Database};
use super::WorkspaceUser;

pub async fn add_user_to_workspace_with_roles(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	roles: &[Uuid],
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	for role_id in roles {
		query!(
			r#"
			INSERT INTO
				workspace_user(
					user_id,
					workspace_id,
					role_id
				)
			VALUES
				($1, $2, $3);
			"#,
			user_id as _,
			workspace_id as _,
			role_id as _,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

pub async fn remove_user_roles_from_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			workspace_id = $1 AND
			user_id = $2;
		"#,
		workspace_id as _,
		user_id as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn list_all_users_with_roles_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<HashMap<Uuid, Vec<Uuid>>, sqlx::Error> {
	let rows = query_as!(
		WorkspaceUser,
		r#"
		SELECT
			user_id as "user_id: _",
			workspace_id as "workspace_id: _",
			role_id as "role_id: _"
		FROM
			workspace_user
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	let mut users = HashMap::new();

	for workspace_user in rows {
		let roles = users.entry(workspace_user.user_id).or_insert(vec![]);
		if !roles.contains(&workspace_user.role_id) {
			roles.push(workspace_user.role_id);
		}
	}

	Ok(users)
}
