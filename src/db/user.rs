use crate::{
	models::{User, UserLogin, UserToSignUp},
	query, query_as,
};
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_users(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL,
			phone_number CHAR(15) UNIQUE NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_email_address (
			email VARCHAR(320) PRIMARY KEY,
			user_id BINARY(16) NOT NULL,
			UNIQUE(email, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
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
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_to_sign_up (
			phone_number CHAR(15) PRIMARY KEY,
			email VARCHAR(320) UNIQUE NOT NULL,
			username VARCHAR(100) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			otp CHAR(4) UNIQUE NOT NULL,
			otp_expiry BIGINT UNSIGNED NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS password_reset_request (
			user_id BINARY(16) PRIMARY KEY,
			token BINARY(64) UNIQUE NOT NULL,
			token_expiry BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_or_email_or_phone_number(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user, user_email_address
		WHERE
			user.id = user_email_address.user_id AND
			(
				user.username = ? OR
				user.phone_number = ? OR
				user_email_address.email = ?
			)
		"#,
		user_id,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row))
}

pub async fn get_user_by_email(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	email: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user, user_email_address
		WHERE
			user.id = user_email_address.user_id AND
			user_email_address.email = ?
		"#,
		email
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row))
}

pub async fn get_user_by_username(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	username: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
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
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row))
}

pub async fn get_user_by_phone_number(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	phone_number: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			phone_number = ?
		"#,
		phone_number
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row))
}

pub async fn set_user_to_be_signed_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	phone_number: &str,
	email: &str,
	username: &str,
	password: &[u8],
	first_name: &str,
	last_name: &str,
	otp: &str,
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			email = ?,
			username = ?,
			password = ?,
			first_name = ?,
			last_name = ?,
			otp = ?,
			otp_expiry = ?;
		"#,
		phone_number,
		email,
		username,
		password,
		first_name,
		last_name,
		otp,
		otp_expiry,
		email,
		username,
		password,
		first_name,
		last_name,
		otp,
		otp_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_user_email_to_sign_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	phone_number: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let rows = query_as!(
		UserToSignUp,
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			phone_number = ?
		"#,
		phone_number
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(row))
}

pub async fn add_email_for_user(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
	email: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_email_address
		VALUES
			(?, ?);
		"#,
		email,
		user_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn delete_user_email_to_be_signed_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			phone_number = ?;
		"#,
		phone_number,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn create_user(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
	username: &str,
	phone_number: &str,
	password: &[u8],
	first_name: &str,
	last_name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user
		VALUES
			(?, ?, ?, ?, ?, ?);
		"#,
		user_id,
		username,
		password,
		phone_number,
		first_name,
		last_name
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
	let rows = query_as!(
		UserLogin,
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

	Ok(Some(row))
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
