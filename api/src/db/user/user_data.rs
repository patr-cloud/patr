use api_models::{models::user::BasicUserInfo, utils::Uuid};
use chrono::{DateTime, Utc};

use crate::{
	db::{PaymentType, Workspace},
	query,
	query_as,
	Database,
};

pub struct User {
	pub id: Uuid,
	pub username: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
	pub dob: Option<DateTime<Utc>>,
	pub bio: Option<String>,
	pub location: Option<String>,
	pub created: DateTime<Utc>,
	pub recovery_email_local: Option<String>,
	pub recovery_email_domain_id: Option<Uuid>,
	pub recovery_phone_country_code: Option<String>,
	pub recovery_phone_number: Option<String>,
	pub workspace_limit: i32,
	pub sign_up_coupon: Option<String>,
}

pub struct PasswordResetRequest {
	pub user_id: Uuid,
	pub token: String,
	pub token_expiry: DateTime<Utc>,
	pub attempts: i32,
}

pub async fn initialize_user_data_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE "user"(
			id UUID CONSTRAINT user_pk PRIMARY KEY,
			username VARCHAR(100) NOT NULL
				CONSTRAINT user_uq_username UNIQUE
				CONSTRAINT user_chk_username_is_valid CHECK(
					/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
					username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
					username NOT LIKE '%..%' AND
					username NOT LIKE '%.-%' AND
					username NOT LIKE '%-.%'
				),
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob TIMESTAMPTZ DEFAULT NULL 
				CONSTRAINT user_chk_dob_is_13_plus CHECK(
					dob IS NULL OR dob < (NOW() - INTERVAL '13 YEARS')
				),
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created TIMESTAMPTZ NOT NULL,
			/* Recovery options */
			recovery_email_local VARCHAR(64)
				CONSTRAINT user_chk_recovery_email_is_lower_case CHECK(
					recovery_email_local = LOWER(recovery_email_local)
				),
			recovery_email_domain_id UUID,
			recovery_phone_country_code CHAR(2)
				CONSTRAINT user_chk_recovery_phone_country_code_is_upper_case CHECK(
					recovery_phone_country_code = UPPER(recovery_phone_country_code)
				),
			recovery_phone_number VARCHAR(15),
			workspace_limit INTEGER NOT NULL,
			sign_up_coupon TEXT,

			CONSTRAINT user_uq_recovery_email_local_recovery_email_domain_id
				UNIQUE(recovery_email_local, recovery_email_domain_id),

			CONSTRAINT user_uq_recovery_phone_country_code_recovery_phone_number
				UNIQUE(recovery_phone_country_code, recovery_phone_number),

			CONSTRAINT user_chk_rcvry_eml_or_rcvry_phn_present CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL
				) OR
				(
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
			user_idx_created
		ON
			"user"
		(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE password_reset_request(
			user_id UUID
				CONSTRAINT password_reset_request_pk PRIMARY KEY
				CONSTRAINT password_reset_request_fk_user_id
					REFERENCES "user"(id),
			token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,
			attempts INT NOT NULL
				CONSTRAINT password_reset_request_attempts_non_negative
					CHECK(attempts >= 0) 
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_user_data_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
        ALTER TABLE "user"
        ADD CONSTRAINT user_fk_sign_up_coupon
        FOREIGN KEY(sign_up_coupon) REFERENCES coupon_code(code);
        "#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_email_or_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = "user".id
		LEFT JOIN
			business_email
		ON
			business_email.user_id = "user".id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = business_email.domain_id
		LEFT JOIN
			user_phone_number
		ON
			user_phone_number.user_id = "user".id
		LEFT JOIN
			phone_number_country_code
		ON
			phone_number_country_code.country_code = user_phone_number.country_code
		WHERE
			"user".username = $1 OR
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				business_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		user_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_username(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		WHERE
			username = $1;
		"#,
		username
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn generate_new_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				"user"
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_god_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Option<Uuid>, sqlx::Error> {
	let uuid = query!(
		r#"
		SELECT
			id as "id: Uuid"
		FROM
			"user"
		ORDER BY
			created
		LIMIT 1;
		"#
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.id);

	Ok(uuid)
}

pub async fn create_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),
	created: &DateTime<Utc>,

	recovery_email_local: Option<&str>,
	recovery_email_domain_id: Option<&Uuid>,

	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	workspace_limit: i32,
	sign_up_coupon: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			"user"(
				id,
				username,
				password,
				first_name,
				last_name,
				dob,
				bio,
				location,
				created,

				recovery_email_local,
				recovery_email_domain_id,

				recovery_phone_country_code,
				recovery_phone_number,

				workspace_limit,

				sign_up_coupon
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				NULL,
				NULL,
				NULL,
				$6,
				
				$7,
				$8,
				
				$9,
				$10,

				$11,

				$12
			);
		"#,
		user_id as _,
		username,
		password,
		first_name,
		last_name,
		created as _,
		recovery_email_local,
		recovery_email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		workspace_limit,
		sign_up_coupon,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_user_data(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	first_name: Option<&str>,
	last_name: Option<&str>,
	dob: Option<&DateTime<Utc>>,
	bio: Option<&str>,
	location: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			first_name = COALESCE($1, first_name),
			last_name = COALESCE($2, last_name),
			dob = COALESCE($3, dob),
			bio = COALESCE($4, bio),
			location = COALESCE($5, location)
		WHERE
			id = $6;
		"#,
		first_name,
		last_name,
		dob as _,
		bio,
		location,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_user_password(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	password: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		password,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_password_reset_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	token_hash: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			password_reset_request(
				user_id,
				token,
				token_expiry,
				attempts
			)
		VALUES
			($1, $2, $3, 0)
		ON CONFLICT(user_id) DO UPDATE SET
			token = EXCLUDED.token,
			token_expiry = EXCLUDED.token_expiry,
			attempts = EXCLUDED.attempts;
		"#,
		user_id as _,
		token_hash,
		token_expiry as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_password_reset_request_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<PasswordResetRequest>, sqlx::Error> {
	query_as!(
		PasswordResetRequest,
		r#"
		SELECT
			user_id as "user_id: _",
			token,
			token_expiry,
			attempts
		FROM
			password_reset_request
		WHERE
			user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_password_reset_request_attempt_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	attempts: i32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			password_reset_request
		SET
			attempts = $2
		WHERE
			user_id = $1;
		"#,
		user_id as _,
		attempts,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_password_reset_request_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			password_reset_request
		WHERE
			user_id = $1;
		"#,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_workspaces_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<Workspace>, sqlx::Error> {
	let res = query!(
		r#"
		SELECT DISTINCT
			workspace.id as "id: Uuid",
			workspace.name::TEXT as "name!: String",
			workspace.super_admin_id as "super_admin_id: Uuid",
			workspace.active,
			workspace.alert_emails as "alert_emails: Vec<String>",
			workspace.payment_type as "payment_type: PaymentType",
			workspace.default_payment_method_id as "default_payment_method_id: String",
			workspace.deployment_limit,
			workspace.static_site_limit,
			workspace.database_limit,
			workspace.managed_url_limit,
			workspace.secret_limit,
			workspace.domain_limit,
			workspace.docker_repository_storage_limit,
			workspace.volume_storage_limit,
			workspace.stripe_customer_id,
			workspace.address_id as "address_id: Uuid",
			workspace.amount_due_in_cents,
			workspace.is_spam
		FROM
			workspace
		LEFT JOIN
			workspace_user
		ON
			workspace.id = workspace_user.workspace_id
		WHERE
			(
				workspace.super_admin_id = $1 OR
				workspace_user.user_id = $1
			) AND
			workspace.deleted IS NULL;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| Workspace {
		id: row.id,
		name: row.name,
		super_admin_id: row.super_admin_id,
		active: row.active,
		alert_emails: row.alert_emails,
		payment_type: row.payment_type,
		default_payment_method_id: row.default_payment_method_id,
		deployment_limit: row.deployment_limit,
		static_site_limit: row.static_site_limit,
		database_limit: row.database_limit,
		managed_url_limit: row.managed_url_limit,
		secret_limit: row.secret_limit,
		domain_limit: row.domain_limit,
		docker_repository_storage_limit: row.docker_repository_storage_limit,
		volume_storage_limit: row.volume_storage_limit,
		stripe_customer_id: row.stripe_customer_id,
		address_id: row.address_id,
		amount_due_in_cents: row.amount_due_in_cents as u64,
		is_spam: row.is_spam,
	})
	.collect();

	Ok(res)
}

pub async fn get_all_workspaces_owned_by_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<Workspace>, sqlx::Error> {
	let res = query!(
		r#"
		SELECT
			id as "id: Uuid",
			name::TEXT as "name!: String",
			super_admin_id as "super_admin_id: Uuid",
			active,
			alert_emails as "alert_emails: Vec<String>",
			payment_type as "payment_type: PaymentType",
			default_payment_method_id as "default_payment_method_id: String",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			volume_storage_limit,
			stripe_customer_id,
			address_id as "address_id: Uuid",
			amount_due_in_cents,
			is_spam
		FROM
			workspace
		WHERE
			super_admin_id = $1 AND
			deleted IS NULL;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| Workspace {
		id: row.id,
		name: row.name,
		super_admin_id: row.super_admin_id,
		active: row.active,
		alert_emails: row.alert_emails,
		payment_type: row.payment_type,
		default_payment_method_id: row.default_payment_method_id,
		deployment_limit: row.deployment_limit,
		static_site_limit: row.static_site_limit,
		database_limit: row.database_limit,
		managed_url_limit: row.managed_url_limit,
		secret_limit: row.secret_limit,
		domain_limit: row.domain_limit,
		docker_repository_storage_limit: row.docker_repository_storage_limit,
		volume_storage_limit: row.volume_storage_limit,
		stripe_customer_id: row.stripe_customer_id,
		address_id: row.address_id,
		amount_due_in_cents: row.amount_due_in_cents as u64,
		is_spam: row.is_spam,
	})
	.collect();

	Ok(res)
}

pub async fn search_for_users(
	connection: &mut <Database as sqlx::Database>::Connection,
	query: &str,
) -> Result<Vec<BasicUserInfo>, sqlx::Error> {
	query_as!(
		BasicUserInfo,
		r#"
		SELECT DISTINCT
			"user".id as "id!: Uuid",
			"user".username as "username!",
			"user".first_name as "first_name!",
			"user".last_name as "last_name!",
			"user".bio as "bio",
			"user".location as "location"
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			"user".id = personal_email.user_id
		LEFT JOIN
			business_email
		ON
			"user".id = personal_email.user_id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = business_email.domain_id
		LEFT JOIN
			user_phone_number
		ON
			"user".id = user_phone_number.user_id
		LEFT JOIN
			phone_number_country_code
		ON
			user_phone_number.country_code = phone_number_country_code.country_code
		WHERE
			(
				CASE
					WHEN LOWER("user".username) = LOWER($1) THEN 100 /* Exact username */
					WHEN COALESCE(
						CASE
							WHEN personal_email.local IS NOT NULL THEN
								LOWER(CONCAT(
									personal_email.local,
									'@',
									domain.name,
									domain.tld
								))
							ELSE NULL
						END,
						CASE
							WHEN business_email.local IS NOT NULL THEN
								LOWER(CONCAT(
									business_email.local,
									'@',
									domain.name,
									domain.tld
								))
							ELSE NULL
						END
					) = LOWER($1) THEN 100 /* Exact email */
					WHEN CONCAT(
						'+',
						phone_number_country_code.phone_code,
						user_phone_number.number
					) = $1 THEN 100 /* Exact phone number */
					WHEN user_phone_number.number = $1 THEN 90 /* Just the phone alone */
					WHEN LOWER(CONCAT(
						"user".first_name,
						' ',
						"user".last_name
					)) = LOWER($1) THEN 90 /* firstName lastName */
					WHEN LOWER(CONCAT(
						"user".last_name,
						' ',
						"user".first_name
					)) = LOWER($1) THEN 90 /* lastName firstName */
					WHEN LOWER("user".first_name) = LOWER($1) THEN 80 /* Only first name */
					WHEN LOWER("user".last_name) = LOWER($1) THEN 80 /* Only last name */

					/* If you search for a part of their username */
					WHEN STARTS_WITH(
						LOWER("user".username),
						SUBSTR(LOWER($1), 0, LENGTH("user".username))
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH("user".username)::REAL
						) * 20
					)

					/* If you search for a part of their name */
					WHEN STARTS_WITH(
						LOWER(CONCAT("user".first_name, ' ', "user".last_name)),
						SUBSTR(
							LOWER($1),
							0,
							LENGTH(CONCAT("user".first_name, ' ', "user".last_name))
						)
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH(CONCAT("user".first_name, ' ', "user".last_name))::REAL
						) * 20
					)

					/* If you search for a part of their name reversed */
					WHEN STARTS_WITH(
						LOWER(CONCAT("user".last_name, ' ', "user".first_name)),
						SUBSTR(
							LOWER($1),
							0,
							LENGTH(CONCAT("user".last_name, ' ', "user".first_name))
						)
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH(CONCAT("user".last_name, ' ', "user".first_name))::REAL
						) * 20
					)

					ELSE 0
				END
			) > 0
		ORDER BY
			1 DESC
		LIMIT 10;
		"#,
		query
	)
	.fetch_all(&mut *connection)
	.await
}
