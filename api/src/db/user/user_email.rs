use api_models::{self, utils::Uuid};
use chrono::{DateTime, Utc};

use super::User;
use crate::{query, query_as, Database};

pub struct PersonalEmailToBeVerified {
	pub local: String,
	pub domain_id: Uuid,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: DateTime<Utc>,
}

pub async fn initialize_user_email_pre(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn initialize_user_email_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE personal_email(
			user_id UUID NOT NULL
				CONSTRAINT personal_email_fk_user_id REFERENCES "user"(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			local VARCHAR(64) NOT NULL
				CONSTRAINT personal_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			CONSTRAINT personal_email_pk PRIMARY KEY(local, domain_id),
			CONSTRAINT personal_email_uq_user_id_local_domain_id
				UNIQUE(user_id, local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			personal_email_idx_user_id
		ON
			personal_email
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE business_email(
			user_id UUID NOT NULL
				CONSTRAINT business_email_fk_user_id REFERENCES "user"(id),
			local VARCHAR(64) NOT NULL
				CONSTRAINT business_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT business_email_fk_domain_id
					REFERENCES workspace_domain(id),
			CONSTRAINT business_email_pk PRIMARY KEY(local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			business_email_idx_user_id
		ON
			business_email
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_unverified_personal_email(
			local VARCHAR(64) NOT NULL
				CONSTRAINT user_unverified_personal_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			user_id UUID NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_user_id
					REFERENCES "user"(id),
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry TIMESTAMPTZ NOT NULL,
			
			CONSTRAINT user_unverified_personal_email_pk
				PRIMARY KEY(local, domain_id),
			CONSTRAINT
				user_unverified_personal_email_uq_user_id_local_domain_id
				UNIQUE(user_id, local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_fk_id_recovery_email_local_recovery_email_domain_id
		FOREIGN KEY(
			id,
			recovery_email_local,
			recovery_email_domain_id
		)
		REFERENCES personal_email(
			user_id,
			local,
			domain_id
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_user_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
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
			"user".sign_up_coupon,
			"user".last_referred,
			"user".referral_click
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
		WHERE
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
			) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	email_local: &str,
	domain_id: &Uuid,
	user_id: &Uuid,
	verification_token: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_personal_email(
				local,
				domain_id,
				user_id,
				verification_token_hash,
				verification_token_expiry
			)
		VALUES
			($1, $2, $3, $4, $5)
		ON CONFLICT(local, domain_id) DO UPDATE SET
			user_id = EXCLUDED.user_id,
			verification_token_hash = EXCLUDED.verification_token_hash,
			verification_token_expiry = EXCLUDED.verification_token_expiry;
		"#,
		email_local,
		domain_id as _,
		user_id as _,
		verification_token,
		token_expiry as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			user_unverified_personal_email.local,
			user_unverified_personal_email.domain_id as "domain_id: _",
			user_unverified_personal_email.user_id as "user_id: _",
			user_unverified_personal_email.verification_token_hash,
			user_unverified_personal_email.verification_token_expiry
		FROM
			user_unverified_personal_email
		INNER JOIN
			domain
		ON
			domain.id = user_unverified_personal_email.domain_id
		WHERE
			user_id = $1 AND
			CONCAT(local, '@', domain.name, '.', domain.tld) = $2;
		"#,
		user_id as _,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_personal_email_to_be_verified_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			user_unverified_personal_email.local,
			user_unverified_personal_email.domain_id as "domain_id: _",
			user_unverified_personal_email.user_id as "user_id: _",
			user_unverified_personal_email.verification_token_hash,
			user_unverified_personal_email.verification_token_expiry
		FROM
			user_unverified_personal_email
		INNER JOIN
			domain
		ON
			domain.id = user_unverified_personal_email.domain_id
		WHERE
			CONCAT(local, '@', domain.name, '.', domain.tld) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_unverified_personal_email
		WHERE
			user_id = $1 AND
			local = $2 AND
			domain_id = $3;
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn add_personal_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_email(
				user_id,
				local,
				domain_id
			)
		VALUES
			($1, $2, $3);
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_business_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			business_email(
				user_id,
				local,
				domain_id
			)
		VALUES
			($1, $2, $3);
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_recovery_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			recovery_email_local = $1,
			recovery_email_domain_id = $2
		WHERE
			id = $3;
		"#,
		email_local,
		domain_id as _,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_personal_emails_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) as "email!: String"
		FROM
			personal_email
		INNER JOIN
			domain
		ON
			personal_email.domain_id = domain.id
		WHERE
			personal_email.user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.email)
	.collect();

	Ok(rows)
}

pub async fn get_personal_email_count_for_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COUNT(personal_email.domain_id) as "count!: i64"
		FROM
			personal_email
		WHERE
			personal_email.domain_id = $1;
		"#,
		domain_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.count as u64)
}

pub async fn get_recovery_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	query!(
		r#"
		SELECT
			CONCAT(
				"user".recovery_email_local,
				'@',
				domain.name,
				'.',
				domain.tld
			) as "email!: String"
		FROM
			"user"
		INNER JOIN
			domain
		ON
			"user".recovery_email_domain_id = domain.id
		WHERE
			"user".id = $1;
		"#,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| row.map(|row| row.email))
}

pub async fn delete_personal_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			personal_email
		WHERE
			user_id = $1 AND
			local = $2 AND
			domain_id = $3;
		"#,
		user_id as _,
		email_local,
		domain_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
