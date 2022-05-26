use api_models::utils::Uuid;

use crate::{query, Database};

pub async fn set_drone_username_and_token_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	login: &str,
	token: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			drone_username = $2,
			drone_token = $3
		WHERE
			id = $1;
		"#,
		workspace_id as _,
		login,
		token
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_drone_username_and_token_from_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			drone_username = NULL AND
			drone_token = NULL
		WHERE
			id = $1;
		"#,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_drone_username_and_token_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Option<(String, String)>, sqlx::Error> {
	let username = query!(
		r#"
		SELECT
			drone_username,
			drone_token
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		workspace_id as _
	)
	.fetch_optional(&mut *connection)
	.await?
	.and_then(|row| Some((row.drone_username?, row.drone_token?)));

	Ok(username)
}
