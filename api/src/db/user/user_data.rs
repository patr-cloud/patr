use crate::prelude::*;

/// Initializes the user tables
#[instrument(skip(connection))]
pub async fn initialize_user_data_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user tables");
	query!(
		r#"
		CREATE TABLE "user"(
			id UUID NOT NULL,
			username VARCHAR(100) NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			/* Recovery options */
			recovery_email_local VARCHAR(64),
			recovery_email_domain_id UUID,
			recovery_phone_country_code CHAR(2),
			recovery_phone_number VARCHAR(15),
			workspace_limit INTEGER NOT NULL,
			password_reset_token TEXT,
			password_reset_token_expiry TIMESTAMPTZ NOT NULL,
			password_reset_attempts INT NOT NULL,
			mfa_secret TEXT
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user data indexes
#[instrument(skip(connection))]
pub async fn initialize_user_data_indexes(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user data indexes");
	query!(
		r#"
		ALTER TABLE "user"
			ADD CONSTRAINT user_pk PRIMARY KEY(id),
			ADD CONSTRAINT user_uq_username UNIQUE(username),
			ADD CONSTRAINT user_uq_recovery_email_local_recovery_email_domain_id
				UNIQUE(recovery_email_local, recovery_email_domain_id),
			ADD CONSTRAINT user_uq_recovery_phone_country_code_recovery_phone_number
				UNIQUE(recovery_phone_country_code, recovery_phone_number);
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

	Ok(())
}

/// Initializes the user data constraints
#[instrument(skip(connection))]
pub async fn initialize_user_data_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user data constraints");
	query!(
		r#"
		ALTER TABLE "user"
			ADD CONSTRAINT user_chk_username_is_valid CHECK(
				/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
				username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
				username NOT LIKE '%..%' AND
				username NOT LIKE '%.-%' AND
				username NOT LIKE '%-.%'
			),
			ADD CONSTRAINT user_chk_recovery_email_is_lower_case CHECK(
				recovery_email_local = LOWER(recovery_email_local)
			),
			ADD CONSTRAINT user_chk_recovery_phone_country_code_is_upper_case CHECK(
				recovery_phone_country_code = UPPER(recovery_phone_country_code)
			),
			ADD CONSTRAINT user_chk_rcvry_eml_or_rcvry_phn_present CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL
				) OR
				(
					recovery_phone_country_code IS NOT NULL AND
					recovery_phone_number IS NOT NULL
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
