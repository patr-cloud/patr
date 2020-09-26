use crate::{
	models::db_mapping::{
		User, UserEmailAddress, UserEmailAddressSignUp, UserLogin, UserToSignUp,
	},
	query, query_as,
};
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_users_pre(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL,
			backup_email VARCHAR(320) UNIQUE NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob BIGINT UNSIGNED DEFAULT NULL,
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created BIGINT UNSIGNED NOT NULL
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

pub async fn initialize_users_post(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	// have two different kinds of email address.
	// One for external email (for personal accounts)
	// and one for organisation emails
	// We can have a complicated constraint like so:
	// Ref: https://stackoverflow.com/a/10273951/3393442
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_email_address (
			type ENUM('personal', 'organisation') NOT NULL,

			# Personal email address
			email_address VARCHAR(320),
			
			# Organisation email address
			email_local VARCHAR(160),
			domain_id BINARY(16),

			user_id BINARY(16) NOT NULL,
			
			UNIQUE(email_address, email_local, domain_id, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(domain_id) REFERENCES domain(id),
			CONSTRAINT CHECK
			(
				(
					type = 'personal' AND
					(
						email_address IS NOT NULL AND
						email_local IS NULL AND
						domain_id IS NULL
					)
				) OR
				(
					type = 'organisation' AND
					(
						email_address IS NULL AND
						email_local IS NOT NULL AND
						domain_id IS NOT NULL
					)
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_unverified_email_address (
			# Personal email address
			email_address VARCHAR(320),
			user_id BINARY(16) NOT NULL,
			verification_token_hash BINARY(64) UNIQUE NOT NULL,
			verification_token_expiry BIGINT UNSIGNED NOT NULL,
			
			PRIMARY KEY(email_address, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE VIEW
			user_email_address_view
		AS
			SELECT
				CASE WHEN (user_email_address.type = 'external') THEN
					user_email_address.email_address
				ELSE
					CONCAT(user_email_address.email_local, '@', domain.name)
				END AS email,
				user_email_address.user_id AS user_id,
				user_email_address.type AS type,
				domain.id AS domain_id,
				domain.name AS domain_name
			FROM
				user_email_address, domain
			WHERE
				user_email_address.domain_id = domain.id;
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_to_sign_up (
			username VARCHAR(100) PRIMARY KEY,
			account_type ENUM('personal', 'organisation') NOT NULL,
			
			# Personal email address OR backup email
			email_address VARCHAR(320) NOT NULL,

			# Organisation email address
			email_local VARCHAR(160),
			domain_name VARCHAR(100),

			password BINARY(64) NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,

			organisation_name VARCHAR(100),

			otp_hash BINARY(64) UNIQUE NOT NULL,
			otp_expiry BIGINT UNSIGNED NOT NULL,

			CONSTRAINT CHECK
			(
				(
					account_type = 'personal' AND
					(
						email_local IS NULL AND
						domain_name IS NULL AND
						organisation_name IS NULL
					)
				) OR
				(
					account_type = 'organisation' AND
					(
						email_local IS NOT NULL AND
						domain_name IS NOT NULL AND
						organisation_name IS NOT NULL
					)
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_or_email(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user, user_email_address_view
		WHERE
			user.id = user_email_address_view.user_id AND
			(
				user.username = ? OR
				user_email_address_view.email = ?
			)
		"#,
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
			user, user_email_address_view
		WHERE
			user.id = user_email_address_view.user_id AND
			user_email_address_view.email = ?
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

pub async fn get_user_by_user_id(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			id = ?
		"#,
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

pub async fn set_user_to_be_signed_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	email: UserEmailAddressSignUp,
	username: &str,
	password: &[u8],
	first_name: &str,
	last_name: &str,
	otp_hash: &[u8],
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	match email {
		UserEmailAddressSignUp::Personal(email) => {
			query!(
				r#"
				INSERT INTO
					user_to_sign_up (
						username,
						account_type,
						
						email_address,
						
						email_local,
						domain_name,
						password,
						first_name,
						last_name,
						
						organisation_name,
						
						otp_hash,
						otp_expiry
					)
				VALUES
					(?, 'personal', ?, NULL, NULL, ?, ?, ?, NULL, ?, ?)
				ON DUPLICATE KEY UPDATE
					account_type = 'personal',
					
					email_address = ?,
					
					email_local = NULL,
					domain_name = NULL,
					
					organisation_name = NULL,
					
					password = ?,
					first_name = ?,
					last_name = ?,
					otp_hash = ?,
					otp_expiry = ?;
				"#,
				username,
				email,
				password,
				first_name,
				last_name,
				otp_hash,
				otp_expiry,
				email,
				password,
				first_name,
				last_name,
				otp_hash,
				otp_expiry
			)
			.execute(connection)
			.await?;
		}
		UserEmailAddressSignUp::Organisation {
			email_local,
			domain_name,
			organisation_name,
			backup_email,
		} => {
			query!(
				r#"
				INSERT INTO
					user_to_sign_up (
						username,
						account_type,
						
						email_address,
						
						email_local,
						domain_name,
						
						password,
						first_name,
						last_name,
						
						organisation_name,
						
						otp_hash,
						otp_expiry
					)
				VALUES
					(?, 'organisation', ?, ?, ?, ?, ?, ?, ?, ?, ?)
				ON DUPLICATE KEY UPDATE
					account_type = 'organisation',
					
					email_address = ?,
					
					email_local = ?,
					domain_name = ?,
					
					password = ?,
					first_name = ?,
					last_name = ?,
					
					organisation_name = ?,
					
					otp_hash = ?,
					otp_expiry = ?;
				"#,
				username,
				backup_email,
				email_local,
				domain_name,
				password,
				first_name,
				last_name,
				organisation_name,
				otp_hash,
				otp_expiry,
				backup_email,
				email_local,
				domain_name,
				password,
				first_name,
				last_name,
				organisation_name,
				otp_hash,
				otp_expiry
			)
			.execute(connection)
			.await?;
		}
	}

	Ok(())
}

pub async fn get_user_email_to_sign_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	username: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
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

	Ok(Some(UserToSignUp {
		username: row.username,
		email: if row.account_type == "personal" {
			UserEmailAddressSignUp::Personal(row.email_address.clone())
		} else if row.account_type == "organisation" {
			UserEmailAddressSignUp::Organisation {
				email_local: row.email_local.unwrap(),
				domain_name: row.domain_name.unwrap(),
				organisation_name: row.organisation_name.unwrap(),
				backup_email: row.email_address.clone(),
			}
		} else {
			panic!("Unknown account_type");
		},
		backup_email: row.email_address,
		password: row.password,
		otp_hash: row.otp_hash,
		otp_expiry: row.otp_expiry,
		first_name: row.first_name,
		last_name: row.last_name,
	}))
}

pub async fn add_personal_email_for_user_to_be_verified(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	email_address: &str,
	user_id: &[u8],
	verification_token: &[u8],
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_email_address (
				email_address,
				user_id,
				verification_token_hash,
				verification_token_expiry
			)
		VALUES
			(?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			verification_token_hash = ?,
			verification_token_expiry = ?;
		"#,
		email_address,
		user_id,
		verification_token,
		token_expiry,
		verification_token,
		token_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_email_for_user(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
	email: UserEmailAddress,
) -> Result<(), sqlx::Error> {
	match email {
		UserEmailAddress::Personal(email) => {
			query!(
				r#"
				INSERT INTO
					user_email_address (
						type,
						email_address,

						email_local,
						domain_id,

						user_id
					)
				VALUES
					('personal', ?, NULL, NULL, ?);
				"#,
				email,
				user_id
			)
			.execute(connection)
			.await?;
		}
		UserEmailAddress::Organisation {
			email_local,
			domain_id,
		} => {
			query!(
				r#"
				INSERT INTO
					user_email_address (
						type,
						email_address,

						email_local,
						domain_id,

						user_id
					)
				VALUES
					('organisation', NULL, ?, ?, ?);
				"#,
				email_local,
				domain_id,
				user_id
			)
			.execute(connection)
			.await?;
		}
	}

	Ok(())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			username = ?;
		"#,
		username,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn create_user(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
	username: &str,
	password: &[u8],
	backup_email: &str,
	first_name: &str,
	last_name: &str,
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user (
				id,
				username,
				password,
				backup_email,
				first_name,
				last_name,
				created
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?);
		"#,
		user_id,
		username,
		password,
		backup_email,
		first_name,
		last_name,
		created
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_user_login(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	refresh_token: &[u8],
	token_expiry: u64,
	user_id: &[u8],
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
	refresh_token: &[u8],
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
	refresh_token: &[u8],
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

pub async fn update_user_data(
	connection: &mut Transaction<PoolConnection<MySqlConnection>>,
	first_name: Option<&str>,
	last_name: Option<&str>,
	dob: Option<&str>,
	bio: Option<&str>,
	location: Option<&str>,
) -> Result<(), sqlx::Error> {
	let params = [
		(first_name, "first_name"),
		(last_name, "last_name"),
		(dob, "dob"),
		(bio, "bio"),
		(location, "location"),
	];

	let param_updates = params
		.iter()
		.filter_map(|(param, name)| {
			if param.is_none() {
				None
			} else {
				Some(format!("{} = ?", name))
			}
		})
		.collect::<Vec<String>>()
		.join(", ");

	let query_string = format!("UPDATE user SET {};", param_updates);
	let mut query = sqlx::query(&query_string);
	for (param, _) in params.iter() {
		if let Some(value) = param {
			query = query.bind(value);
		}
	}
	query.execute(connection).await?;

	Ok(())
}
