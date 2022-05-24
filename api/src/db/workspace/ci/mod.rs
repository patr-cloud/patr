use api_models::utils::Uuid;

use crate::{query, Database};

pub async fn add_drone_username_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	drone_username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			UPDATE workspace
				SET drone_username = $1
			WHERE id = $2
		"#,
		drone_username,
		workspace_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_drone_username(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	let username = query!(
		r#"
		SELECT drone_username as "drone_username!: String"
			FROM workspace
		WHERE 
			id = $1;
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.drone_username);

	Ok(username)
}

pub async fn delete_user_by_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE workspace
			SET drone_username = NULL
		WHERE 
			id = $1;
		"#,
		workspace_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_drone_access_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_login: &str,
) -> Result<Option<String>, sqlx::Error> {
	let token = query!(
		r#"
			SELECT user_hash as "user_login!: String"
				FROM users
			WHERE 
				user_login = $1;
		"#,
		user_login
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.user_login);

	Ok(token)
}
