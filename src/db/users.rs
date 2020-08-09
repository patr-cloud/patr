use crate::{models::user::User, query};
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn get_user(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			users
		WHERE
			username = ? OR
			email = ?
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	if rows.len() == 0 {
		return Ok(None);
	}

	Ok(Some(User::from(
		rows[0].userId.clone(),
		rows[0].username.clone(),
		rows[0].password.clone(),
		rows[0].email.clone(),
	)))
}
