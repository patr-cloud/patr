use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	models::db_mapping::{
		Organisation,
		PasswordResetRequest,
		PersonalEmailToBeVerified,
		User,
		UserEmailAddress,
		UserEmailAddressSignUp,
		UserLogin,
		UserToSignUp,
	},
	query,
	query_as,
	utils,
};

pub async fn initialize_users_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password TEXT NOT NULL,
			backup_email_type ENUM('personal','organisation'),
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob BIGINT UNSIGNED DEFAULT NULL,
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created BIGINT UNSIGNED NOT NULL,
			backup_email_local VARCHAR(54),
			backup_email_domain_id BINARY(16),
			backup_phone_number_country_code VARCHAR(4),
			backup_phone_number VARCHAR(15),
			CONSTRAINT email_always_personal CHECK (
				backup_email_type='personal'
			),
			CONSTRAINT recovery CHECK (
				(
					backup_email_local IS NULL 
					AND
					backup_email_domain IS NULL
				)
				OR
				(
					recovery_phone_number IS NULL
					AND
					recovery_phone_number_country_code IS NULL
				)
			),
			FOREIGN KEY(id, backup_email_local, backup_email_domain_id) REFERENCES personal_email(user_id, email_local, domain_id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_login (
			login_id BINARY(16) PRIMARY KEY,
			refresh_token TEXT NOT NULL,
			token_expiry BIGINT UNSIGNED NOT NULL,
			user_id BINARY(16) NOT NULL,
			last_login BIGINT UNSIGNED NOT NULL,
			last_activity BIGINT UNSIGNED NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS password_reset_request (
			user_id BINARY(16) PRIMARY KEY,
			token TEXT NOT NULL,
			token_expiry BIGINT UNSIGNED NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_users_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	// have two different kinds of email address.
	// One for external email (for personal accounts)
	// and one for organisation emails
	// We can have a complicated constraint like so:
	// Ref: https://stackoverflow.com/a/10273951/3393442
	// query!(
	// 	r#"
	// 	CREATE TABLE IF NOT EXISTS user_email_address (
	// 		type ENUM('personal', 'organisation') NOT NULL,

	// 		/* Personal email address */
	// 		email_address VARCHAR(320),

	// 		/* Organisation email address */
	// 		email_local VARCHAR(160),
	// 		domain_id BINARY(16),

	// 		user_id BINARY(16) NOT NULL,

	// 		UNIQUE(email_address, email_local, domain_id, user_id),
	// 		FOREIGN KEY(user_id) REFERENCES user(id),
	// 		FOREIGN KEY(domain_id) REFERENCES domain(id),
	// 		CONSTRAINT CHECK
	// 		(
	// 			(
	// 				type = 'personal' AND
	// 				(
	// 					email_address IS NOT NULL AND
	// 					email_local IS NULL AND
	// 					domain_id IS NULL
	// 				)
	// 			) OR
	// 			(
	// 				type = 'organisation' AND
	// 				(
	// 					email_address IS NULL AND
	// 					email_local IS NOT NULL AND
	// 					domain_id IS NOT NULL
	// 				)
	// 			)
	// 		)
	// 	);
	// 	"#
	// )
	// .execute(&mut *transaction)
	// .await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS personal_email (
			user_id BINARY(16) NOT NULL,
			email_local VARCHAR(54),
			domain_id BINARY(16),
			PRIMARY KEY(email_local, domain_id),
			CONSTRAINT FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// more details can be added here in the organisation table
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_email (
			user_id BINARY(16) NOT NULL,
			email_local VARCHAR(54),
			domain_id BINARY(16),
			PRIMARY KEY(email_local, domain_id),
			CONSTRAINT FOREIGN KEY(user_id) REFERENCES USER(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// altering the table to add the foreign key reference of
	query!(
		r#"
		ALTER TABLE personal_email
		ADD CONSTRAINT 
		FOREIGN KEY(domain_id) REFERENCES personal_domain(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE organisation_email
		ADD CONSTRAINT 
		FOREIGN KEY(domain_id) REFERENCES organisation_domain(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// we might need to add contraints for every country because every country
	// has a different length of mobile number and coutnry code
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_contact_number (
			user_id BINARY(16) NOT NULL,
			country_code VARCHAR(4) NOT NULL,
			number VARCHAR(15) PRIMARY KEY,
			PRIMARY KEY(country_code, number),
			CONSTRAINT FOREIGN KEY(user_id) REFERENCES user(id),
			CONSTRAINT country_code_check CHECK(CHAR_LENGTH(country_code) >= 2 AND CHAR_LENGTH(country_code) <= 4),
			CONSTRAINT phone_number_check CHECK(CHAR_LENGTH(number) >= 7 AND CHAR_LENGTH(number) <= 15)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE user
		ADD CONSTRAINT 
		FOREIGN KEY(backup_email_local, backup_email_domain_id) REFERENCES personal_email_address(email_local,domain_id),
		ADD CONSTRAINT 
		FOREIGN KEY(recovery_phone_number, recovery_phone_number_country_code) REFERENCES user_contact_number(number, country_code);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_unverified_email_address (
			/* Personal email address */
			email_address VARCHAR(320),
			user_id BINARY(16) NOT NULL,
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry BIGINT UNSIGNED NOT NULL,
			
			PRIMARY KEY(email_address, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// query!(
	// 	r#"
	// 	/*CREATE VIEW
	// 		user_email_address_view
	// 	AS
	// 		SELECT
	// 			CASE WHEN (user_email_address.type = 'personal') THEN
	// 				user_email_address.email_address
	// 			ELSE
	// 				CONCAT(user_email_address.email_local, '@', domain.name)
	// 			END AS email,
	// 			user_email_address.user_id AS user_id,
	// 			user_email_address.type AS type,
	// 			domain.id AS domain_id,
	// 			domain.name AS domain_name
	// 		FROM
	// 			user_email_address
	// 		LEFT JOIN
	// 			domain
	// 		ON
	// 			user_email_address.domain_id = domain.id;*/
	// 	CREATE VIEW
	// 		user_email_address_view
	// 	AS
	// 		SELECT CONCAT(personal)
	// 			FROM personal_email WHERE
	// 	"#
	// )
	// .execute(&mut *transaction)
	// .await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_to_sign_up (
			username VARCHAR(100) PRIMARY KEY,
			account_type ENUM('personal', 'organisation') NOT NULL,
			
			/* Personal email address OR backup email */
			email_address VARCHAR(320) NOT NULL,

			/* Organisation email address */
			email_local VARCHAR(160),
			domain_name VARCHAR(100),

			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,

			organisation_name VARCHAR(100),

			otp_hash TEXT NOT NULL,
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
	connection: &mut Transaction<'_, MySql>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = user.id
		WHERE
			user.username = ? OR
			CONCAT(personal_email.email_local,'@',personal_email.email_domain) = ?;
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn generate_new_user_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			User,
			r#"
			SELECT
				*
			FROM
				user
			WHERE
				id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_god_user_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Option<Uuid>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		ORDER BY
			created
		DESC
		LIMIT 1;
		"#
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();
	let id = Uuid::from_slice(row.id.as_ref())
		.expect("unable to unwrap UUID from Vec<u8>");

	Ok(Some(id))
}

// This only retreives user using personal email address/backup email address
pub async fn get_user_by_email(
	connection: &mut Transaction<'_, MySql>,
	email: &str,
) -> Result<Option<User>, sqlx::Error> {
	let email_local_domain: Vec<&str> = email.split('@').collect();

	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user
		LEFT JOIN
			personal_email
		ON
		personal_email.user_id = user.id
		WHERE
			personal_email.email_local = ?
		AND
			personal_email.email_domain = ?
		"#,
		email_local_domain[0],
		email_local_domain[1]
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_username(
	connection: &mut Transaction<'_, MySql>,
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

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_user_id(
	connection: &mut Transaction<'_, MySql>,
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

	Ok(rows.into_iter().next())
}

pub async fn set_user_to_be_signed_up(
	connection: &mut Transaction<'_, MySql>,
	email: UserEmailAddressSignUp,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),
	otp_hash: &str,
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	match email {
		UserEmailAddressSignUp::Personal(email) => {
			query!(
				r#"
				INSERT INTO
					user_to_sign_up
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
					user_to_sign_up
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

pub async fn get_user_to_sign_up_by_username(
	connection: &mut Transaction<'_, MySql>,
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

pub async fn get_user_to_sign_up_by_organisation_name(
	connection: &mut Transaction<'_, MySql>,
	organisation_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let now = utils::get_current_time();
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			organisation_name = ? AND
			otp_expiry < ?
		"#,
		organisation_name,
		now,
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(UserToSignUp {
		username: row.username,
		email: if row.account_type == "organisation" {
			UserEmailAddressSignUp::Organisation {
				email_local: row.email_local.unwrap(),
				domain_name: row.domain_name.unwrap(),
				organisation_name: row.organisation_name.unwrap(),
				backup_email: row.email_address.clone(),
			}
		} else {
			panic!("account_type wasn't organisation for an organisation");
		},
		backup_email: row.email_address,
		password: row.password,
		otp_hash: row.otp_hash,
		otp_expiry: row.otp_expiry,
		first_name: row.first_name,
		last_name: row.last_name,
	}))
}

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
	email: &str,
	user_id: &[u8],
	verification_token: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_email_address
		VALUES
			(?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			verification_token_hash = ?,
			verification_token_expiry = ?;
		"#,
		email,
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

pub async fn get_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	let rows = query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			*
		FROM
			user_unverified_email_address
		WHERE
			user_id = ? AND
			email_address = ?;
		"#,
		user_id,
		email
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn add_email_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	email: UserEmailAddress,
) -> Result<(), sqlx::Error> {
	match email {
		UserEmailAddress::Personal { email, domain_id } => {
			let email_local_domain: Vec<&str> = email.split('@').collect();

			query!(
				r#"
				INSERT INTO
					personal_email
				VALUES
					(?, ?, ?, ?);
				"#,
				user_id,
				email_local_domain[0],
				email_local_domain[1],
				domain_id
			)
			.execute(connection)
			.await?;
		}
		UserEmailAddress::Organisation {
			email_local,
			domain_id,
			domain_name,
		} => {
			query!(
				r#"
				INSERT INTO
					organisation_email
				VALUES
					(?, ?, ?, ?);
				"#,
				user_id,
				email_local,
				domain_name,
				domain_id
			)
			.execute(connection)
			.await?;
		}
	}

	Ok(())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	username: &str,
	password: &str,
	backup_email: &str,
	recovery_phone_number: &str,
	(first_name, last_name): (&str, &str),
	created: u64,
) -> Result<(), sqlx::Error> {
	let email_local_domain: Vec<&str> = backup_email.split('@').collect();

	query!(
		r#"
		INSERT INTO
			user
		VALUES
			(?, ?, ?, ?, ?, ?, 'personal', ?, ?, NULL, NULL, NULL, ?);
		"#,
		user_id,
		username,
		password,
		email_local_domain[0],
		email_local_domain[1],
		recovery_phone_number,
		first_name,
		last_name,
		created
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_user_login(
	connection: &mut Transaction<'_, MySql>,
	login_id: &[u8],
	refresh_token: &str,
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
			(?, ?, ?, ?, ?, ?);
		"#,
		login_id,
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
	connection: &mut Transaction<'_, MySql>,
	login_id: &[u8],
) -> Result<Option<UserLogin>, sqlx::Error> {
	let rows = query_as!(
		UserLogin,
		r#"
		SELECT * FROM
			user_login
		WHERE
			login_id = ?;
		"#,
		login_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn generate_new_login_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			login_id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			UserLogin,
			r#"
			SELECT
				*
			FROM
				user_login
			WHERE
				login_id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_all_logins_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<Vec<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			user_id = ?;
		"#,
		user_id
	)
	.fetch_all(connection)
	.await
}

pub async fn set_refresh_token_expiry(
	connection: &mut Transaction<'_, MySql>,
	login_id: &[u8],
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
			login_id = ?;
		"#,
		token_expiry,
		last_activity,
		login_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn update_user_data(
	connection: &mut Transaction<'_, MySql>,
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

pub async fn update_user_password(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	password: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user
		SET
			password = ?
		WHERE
			id = ?;
		"#,
		password,
		user_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_password_reset_request(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	token_hash: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			password_reset_request
		VALUES
			(?, ?, ?)
		ON DUPLICATE KEY UPDATE
			token = ?,
			token_expiry = ?;
		"#,
		user_id,
		token_hash,
		token_expiry,
		token_hash,
		token_expiry,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_password_reset_request_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<Option<PasswordResetRequest>, sqlx::Error> {
	let rows = query_as!(
		PasswordResetRequest,
		r#"
		SELECT
			*
		FROM
			password_reset_request
		WHERE
			user_id = ?;
		"#,
		user_id,
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn delete_password_reset_request_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			password_reset_request
		WHERE
			user_id = ?;
		"#,
		user_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_all_organisations_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<Vec<Organisation>, sqlx::Error> {
	let organisations = query_as!(
		Organisation,
		r#"
		SELECT DISTINCT
			organisation.id,
			organisation.name,
			organisation.super_admin_id,
			organisation.active as `active!: bool`,
			organisation.created
		FROM
			organisation
		LEFT JOIN
			organisation_user
		ON
			organisation.id = organisation_user.organisation_id
		WHERE
			organisation.super_admin_id = ? OR
			organisation_user.user_id = ?;
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(organisations)
}
