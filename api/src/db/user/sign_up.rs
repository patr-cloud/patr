use api_models::utils::{ResourceType, Uuid};
use chrono::{DateTime, Utc};

use crate::{query, query_as, Database};

pub struct UserToSignUp {
	pub username: String,
	pub account_type: ResourceType,

	pub password: String,
	pub first_name: String,
	pub last_name: String,

	pub recovery_email_local: Option<String>,
	pub recovery_email_domain_id: Option<Uuid>,

	pub recovery_phone_country_code: Option<String>,
	pub recovery_phone_number: Option<String>,

	pub business_email_local: Option<String>,
	pub business_domain_name: Option<String>,
	pub business_name: Option<String>,

	pub otp_hash: String,
	pub otp_expiry: DateTime<Utc>,

	pub coupon_code: Option<String>,
}

pub struct CouponCode {
	pub code: String,
	pub credits_in_cents: u64,
	pub expiry: Option<DateTime<Utc>>,
	pub uses_remaining: Option<u32>,
	pub is_referral: bool,
}

pub async fn initialize_user_sign_up_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE coupon_code(
			code TEXT CONSTRAINT coupon_code_pk PRIMARY KEY,
			credits_in_cents BIGINT NOT NULL
				CONSTRAINT coupon_code_chk_credits_in_cents_positive
					CHECK (credits_in_cents >= 0),
			expiry TIMESTAMPTZ,
			uses_remaining INTEGER,
			is_referral BOOL NOT NULL DEFAULT false
			CONSTRAINT
				coupon_code_chk_uses_remaining_positive CHECK(
					uses_remaining IS NULL OR uses_remaining > 0
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_user_sign_up_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE user_to_sign_up(
			username VARCHAR(100) CONSTRAINT user_to_sign_up_pk PRIMARY KEY
				CONSTRAINT user_to_sign_up_chk_username_is_valid CHECK(
					/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
					username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
					username NOT LIKE '%..%' AND
					username NOT LIKE '%.-%' AND
					username NOT LIKE '%-.%'
				),
			account_type RESOURCE_OWNER_TYPE NOT NULL,

			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			
			/* Personal email address OR recovery email */
			recovery_email_local VARCHAR(64)
				CONSTRAINT user_to_sign_up_chk_recovery_email_is_lower_case CHECK(
					recovery_email_local = LOWER(recovery_email_local)
				),
			recovery_email_domain_id UUID
				CONSTRAINT user_to_sign_up_fk_recovery_email_domain_id
					REFERENCES personal_domain(id),

			recovery_phone_country_code CHAR(2)
				CONSTRAINT user_to_sign_up_fk_recovery_phone_country_code
					REFERENCES phone_number_country_code(country_code)
				CONSTRAINT user_to_sign_up_chk_recovery_phone_country_code_upper_case CHECK(
					recovery_phone_country_code = UPPER(recovery_phone_country_code)
				),
			recovery_phone_number VARCHAR(15)
				CONSTRAINT user_to_sign_up_chk_phone_number_valid CHECK(
					LENGTH(recovery_phone_number) >= 7 AND
					LENGTH(recovery_phone_number) <= 15 AND
					CAST(recovery_phone_number AS BIGINT) > 0
				),

			/* Workspace email address */
			business_email_local VARCHAR(64)
				CONSTRAINT
					user_to_sign_up_chk_business_email_local_is_lower_case
						CHECK(
							business_email_local = LOWER(business_email_local)
						),
			business_domain_name TEXT
				CONSTRAINT user_to_sign_up_chk_business_domain_name_is_valid
					CHECK(
						business_domain_name ~
							'^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
					),
			business_domain_tld TEXT
				CONSTRAINT
					user_to_sign_up_chk_business_domain_tld_is_length_valid
						CHECK(
							LENGTH(business_domain_tld) >= 2 AND
							LENGTH(business_domain_tld) <= 6
						)
				CONSTRAINT user_to_sign_up_chk_business_domain_tld_is_valid
					CHECK(
						business_domain_tld ~
							'^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
					)
				CONSTRAINT user_to_sign_up_fk_business_domain_tld
					REFERENCES domain_tld(tld),
			business_name VARCHAR(100)
				CONSTRAINT user_to_sign_up_chk_business_name_is_lower_case
					CHECK(business_name = LOWER(business_name)),
			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL,
			coupon_code TEXT CONSTRAINT user_to_sign_up_fk_coupon_code
				REFERENCES coupon_code(code),

			CONSTRAINT user_to_sign_up_chk_max_domain_name_length CHECK(
				(LENGTH(business_domain_name) + LENGTH(business_domain_tld)) < 255
			),
			CONSTRAINT user_to_sign_up_chk_business_details_valid CHECK(
				(
					account_type = 'personal' AND
					(
						business_email_local IS NULL AND
						business_domain_name IS NULL AND
						business_domain_tld IS NULL AND
						business_name IS NULL
					)
				) OR
				(
					account_type = 'business' AND
					(
						business_email_local IS NOT NULL AND
						business_domain_name IS NOT NULL AND
						business_domain_tld IS NOT NULL AND
						business_name IS NOT NULL
					)
				)
			),
			CONSTRAINT user_to_sign_up_chk_recovery_details CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL AND
					recovery_phone_country_code IS NULL AND
					recovery_phone_number IS NULL
				) OR
				(
					recovery_email_local IS NULL AND
					recovery_email_domain_id IS NULL AND
					recovery_phone_country_code IS NOT NULL AND
					recovery_phone_number IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_otp_expiry
		ON
			user_to_sign_up
		(otp_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_username_otp_expiry
		ON
			user_to_sign_up
		(username, otp_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn set_personal_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	email_local: Option<&str>,
	email_domain_id: Option<&Uuid>,
	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	otp_hash: &str,
	otp_expiry: &DateTime<Utc>,

	coupon_code: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up(
				username,
				account_type,
				password,
				first_name,
				last_name,
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code,
				recovery_phone_number,
				business_email_local,
				business_domain_name,
				business_domain_tld,
				business_name,
				otp_hash,
				otp_expiry,
				coupon_code
			)
		VALUES
			(
				$1,
				'personal',
				
				$2,
				$3,
				$4,
				
				$5,
				$6,
				
				$7,
				$8,
				
				NULL,
				NULL,
				NULL,
				NULL,
				
				$9,
				$10,

				$11
			)
		ON CONFLICT(username) DO UPDATE SET
			account_type = 'personal',

			password = EXCLUDED.password,
			first_name = EXCLUDED.first_name,
			last_name = EXCLUDED.last_name,
			
			recovery_email_local = EXCLUDED.recovery_email_local,
			recovery_email_domain_id = EXCLUDED.recovery_email_domain_id,

			recovery_phone_country_code = EXCLUDED.recovery_phone_country_code,
			recovery_phone_number = EXCLUDED.recovery_phone_number,
			
			business_email_local = NULL,
			business_domain_name = NULL,
			business_name = NULL,
			
			otp_hash = EXCLUDED.otp_hash,
			otp_expiry = EXCLUDED.otp_expiry,
			
			coupon_code = EXCLUDED.coupon_code;
		"#,
		username,
		password,
		first_name,
		last_name,
		email_local,
		email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		otp_hash,
		otp_expiry as _,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_business_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	recovery_email_local: Option<&str>,
	recovery_email_domain_id: Option<&Uuid>,
	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	business_email_local: &str,
	business_domain_name: &str,
	business_domain_tld: &str,
	business_name: &str,

	otp_hash: &str,
	otp_expiry: &DateTime<Utc>,

	coupon_code: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up(
				username,
				account_type,
				password,
				first_name,
				last_name,
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code,
				recovery_phone_number,
				business_email_local,
				business_domain_name,
				business_domain_tld,
				business_name,
				otp_hash,
				otp_expiry,
				coupon_code
			)
		VALUES
			(
				$1,
				'business',
				
				$2,
				$3,
				$4,
				
				$5,
				$6,
				
				$7,
				$8,
				
				$9,
				$10,
				$11,
				$12,

				$13,
				$14,

				$15
			)
		ON CONFLICT(username) DO UPDATE SET
			account_type = 'business',

			password = EXCLUDED.password,
			first_name = EXCLUDED.first_name,
			last_name = EXCLUDED.last_name,
			
			recovery_email_local = EXCLUDED.recovery_email_local,
			recovery_email_domain_id = EXCLUDED.recovery_email_domain_id,

			recovery_phone_country_code = EXCLUDED.recovery_phone_country_code,
			recovery_phone_number = EXCLUDED.recovery_phone_number,
			
			business_email_local = EXCLUDED.business_email_local,
			business_domain_name = EXCLUDED.business_domain_name,
			business_domain_tld = EXCLUDED.business_domain_tld,
			business_name = EXCLUDED.business_name,
			
			otp_hash = EXCLUDED.otp_hash,
			otp_expiry = EXCLUDED.otp_expiry,
			
			coupon_code = EXCLUDED.coupon_code;
		"#,
		username,
		password,
		first_name,
		last_name,
		recovery_email_local,
		recovery_email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		business_email_local,
		business_domain_name,
		business_domain_tld,
		business_name,
		otp_hash,
		otp_expiry as _,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_user_to_sign_up_by_username(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			username = $1;
		"#,
		username
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			recovery_phone_country_code = $1 AND
			recovery_phone_number = $2;
		"#,
		country_code,
		phone_number
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			user_to_sign_up.username,
			user_to_sign_up.account_type as "account_type: _",
			user_to_sign_up.password,
			user_to_sign_up.first_name,
			user_to_sign_up.last_name,
			user_to_sign_up.recovery_email_local,
			user_to_sign_up.recovery_email_domain_id as "recovery_email_domain_id: _",
			user_to_sign_up.recovery_phone_country_code,
			user_to_sign_up.recovery_phone_number,
			user_to_sign_up.business_email_local,
			CASE
				WHEN user_to_sign_up.business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(
						user_to_sign_up.business_domain_name,
						'.',
						user_to_sign_up.business_domain_tld
					)
			END as "business_domain_name: _",
			user_to_sign_up.business_name,
			user_to_sign_up.otp_hash,
			user_to_sign_up.otp_expiry,
			user_to_sign_up.coupon_code
		FROM
			user_to_sign_up
		INNER JOIN
			domain
		ON
			domain.id = user_to_sign_up.recovery_email_domain_id
		WHERE
			CONCAT(
				user_to_sign_up.recovery_email_local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_business_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	business_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			business_name = $1;
		"#,
		business_name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_business_domain_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	business_domain_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			business_domain_name = $1;
		"#,
		business_domain_name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_user_to_sign_up_with_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	verification_token: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_to_sign_up
		SET
			otp_hash = $1,
			otp_expiry = $2
		WHERE
			username = $3;
		"#,
		verification_token,
		token_expiry as _,
		username
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			username = $1;
		"#,
		username,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_sign_up_coupon_by_code(
	connection: &mut <Database as sqlx::Database>::Connection,
	coupon_code: &str,
) -> Result<Option<CouponCode>, sqlx::Error> {
	let row = query!(
		r#"
		SELECT
			*
		FROM
			coupon_code
		WHERE
			code = $1;
		"#,
		coupon_code,
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| CouponCode {
		code: row.code,
		credits_in_cents: row.credits_in_cents as u64,
		uses_remaining: row.uses_remaining.map(|uses| uses as u32),
		expiry: row.expiry,
		is_referral: row.is_referral,
	});

	Ok(row)
}

pub async fn update_coupon_code_uses_remaining(
	connection: &mut <Database as sqlx::Database>::Connection,
	coupon_code: &str,
	uses_remaining: u32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			coupon_code
		SET
			uses_remaining = $1
		WHERE
			code = $2;
		"#,
		uses_remaining as i32,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_user_coupon_code(
	connection: &mut <Database as sqlx::Database>::Connection,
	referral_code: &str,
	credits: u32,
	is_referral: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			coupon_code(
				code,
				credits_in_cents,
				is_referral
			)
		VALUES
		(
			$1,
			$2,
			$3
		);
		"#,
		referral_code,
		credits as i32,
		is_referral
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
