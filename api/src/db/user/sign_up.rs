use crate::prelude::*;

/// Initializes the user sign up tables
#[instrument(skip(connection))]
pub async fn initialize_user_sign_up_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE user_to_sign_up(
			username VARCHAR(100) NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			
			/* Personal email address OR recovery email */
			recovery_email_local VARCHAR(64),
			recovery_email_domain_id UUID,
			recovery_phone_country_code CHAR(2),
			recovery_phone_number VARCHAR(15),

			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user sign up constraints
#[instrument(skip(connection))]
pub async fn initialize_user_sign_up_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE user_to_sign_up
			ADD CONSTRAINT user_to_sign_up_pk PRIMARY KEY(username),
			ADD CONSTRAINT user_to_sign_up_chk_username_is_valid CHECK(
				/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
				username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
				username NOT LIKE '%..%' AND
				username NOT LIKE '%.-%' AND
				username NOT LIKE '%-.%'
			),
			ADD CONSTRAINT user_to_sign_up_chk_recovery_email_is_lower_case CHECK(
				recovery_email_local = LOWER(recovery_email_local)
			),
			ADD CONSTRAINT user_to_sign_up_fk_recovery_email_domain_id
				FOREIGN KEY(recovery_email_domain_id) REFERENCES personal_domain(id),
			ADD CONSTRAINT user_to_sign_up_fk_recovery_phone_country_code
				FOREIGN KEY(recovery_phone_country_code)
					REFERENCES phone_number_country_code(country_code),
			ADD CONSTRAINT user_to_sign_up_chk_recovery_phone_country_code_upper_case CHECK(
				recovery_phone_country_code = UPPER(recovery_phone_country_code)
			),
			ADD CONSTRAINT user_to_sign_up_chk_phone_number_valid CHECK(
				LENGTH(recovery_phone_number) >= 7 AND
				LENGTH(recovery_phone_number) <= 15 AND
				CAST(recovery_phone_number AS BIGINT) > 0
			),
			ADD	CONSTRAINT user_to_sign_up_chk_recovery_details CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL AND
					recovery_phone_country_code IS NULL AND
					recovery_phone_number IS NULL
				) OR (
					recovery_email_local IS NULL AND
					recovery_email_domain_id IS NULL AND
					recovery_phone_country_code IS NOT NULL AND
					recovery_phone_number IS NOT NULL
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
