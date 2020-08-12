use crate::{
	models::user::{User, UserLogin},
	query,
};
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
		CREATE TABLE IF NOT EXISTS user_login (
			refresh_token BINARY(16) PRIMARY KEY,
			token_expiry BIGINT UNSIGNED NOT NULL,
			user_id BINARY(16) NOT NULL,
			last_login BIGINT UNSIGNED NOT NULL,
			last_activity BIGINT UNSIGNED NOT NULL,
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
			token_expiry BIGINT UNSIGNED NOT NULL
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
			token_expiry BIGINT UNSIGNED NOT NULL,
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

pub async fn add_user_login(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	refresh_token: Vec<u8>,
	token_expiry: u64,
	user_id: Vec<u8>,
	last_login: u64,
	last_activity: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_login
		VALUES
			(?, ?, ?, ?, ?);
		"#,
		refresh_token,
		token_expiry,
		user_id,
		last_login,
		last_activity
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_user_login(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	refresh_token: Vec<u8>,
) -> Result<Option<UserLogin>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT * FROM
			user_login
		WHERE
			refresh_token = ?;
		"#,
		refresh_token
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}

	let row = rows.into_iter().next().unwrap();

	Ok(Some(UserLogin::from(
		row.refresh_token,
		row.token_expiry,
		row.user_id,
		row.last_login,
		row.last_activity,
	)))
}

pub async fn set_refresh_token_expiry(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	refresh_token: Vec<u8>,
	last_activity: u64,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_login
		SET
			token_expiry = ?,
			last_activity = ?
		WHERE
			refresh_token = ?;
		"#,
		token_expiry,
		last_activity,
		refresh_token
	)
	.execute(connection)
	.await?;

	Ok(())
}
