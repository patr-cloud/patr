use crate::{models::user::User, query};
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_users(
	mut transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS user (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			email VARCHAR(320) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL
		);
		"#
	)
	.execute(&mut transaction)
	.await?;

	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS login (
			auth_token BINARY(16) PRIMARY KEY,
			auth_exp BIGINT NOT NULL,
			user_id BINARY(16) NOT NULL,
			last_login BIGINT NOT NULL,
			last_activity BIGINT,
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut transaction)
	.await?;

	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS emails_to_be_verified (
			email VARCHAR(320) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL,
			token BINARY(64) UNIQUE NOT NULL,
			token_expiry BIGINT NOT NULL
		);
		"#
	)
	.execute(&mut transaction)
	.await?;

	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS password_reset_requests (
			user_id BINARY(16) PRIMARY KEY,
			token BINARY(64) UNIQUE NOT NULL,
			token_expiry BIGINT NOT NULL,
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut transaction)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_or_email(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			username = ? OR
			email = ?
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	Ok(Some(User::from(
		rows[0].id.clone(),
		rows[0].username.clone(),
		rows[0].password.clone(),
		rows[0].email.clone(),
	)))
}

pub async fn get_user_by_email(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	email: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			email = ?
		"#,
		email
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	Ok(Some(User::from(
		rows[0].id.clone(),
		rows[0].username.clone(),
		rows[0].password.clone(),
		rows[0].email.clone(),
	)))
}

pub async fn get_user_by_username(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	username: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			username = ?
		"#,
		username
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	Ok(Some(User::from(
		rows[0].id.clone(),
		rows[0].username.clone(),
		rows[0].password.clone(),
		rows[0].email.clone(),
	)))
}

pub async fn set_user_email_to_be_verified(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	email: &str,
	username: &str,
	password: &[u8],
	token_hash: &[u8],
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			emails_to_be_verified
		VALUES
			(?, ?, ?, ?, ?);
		"#,
		email,
		username,
		password,
		token_hash,
		token_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}
