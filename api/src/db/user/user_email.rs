use crate::prelude::*;

/// Initializes the user email tables
#[instrument(skip(connection))]
pub async fn initialize_user_email_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user email tables");
	query!(
		r#"
		CREATE TABLE user_email(
			user_id UUID,
			local VARCHAR(64) NOT NULL,
			domain_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_unverified_email(
			local VARCHAR(64) NOT NULL,
			domain_id UUID NOT NULL,
			user_id UUID NOT NULL,
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user email indices
#[instrument(skip(connection))]
pub async fn initialize_user_email_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user email indices");
	query!(
		r#"
		ALTER TABLE user_email
			ADD CONSTRAINT user_email_pk PRIMARY KEY(local, domain_id),
			ADD CONSTRAINT user_email_uq_user_id_local_domain_id UNIQUE(
				user_id, local, domain_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_email_idx_user_id
		ON
			user_email
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_email
			ADD CONSTRAINT user_unverified_email_pk PRIMARY KEY(local, domain_id),
			ADD CONSTRAINT user_unverified_email_uq_user_id_local_domain_id UNIQUE(
				user_id, local, domain_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user email constraints
#[instrument(skip(connection))]
pub async fn initialize_user_email_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user email constraints");
	query!(
		r#"
		ALTER TABLE user_email
			ADD CONSTRAINT user_email_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id) DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT user_email_chk_local_is_lower_case CHECK(
				local = LOWER(local)
			),
			ADD CONSTRAINT user_email_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES personal_domain(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_email
			ADD CONSTRAINT user_unverified_email_chk_local_is_lower_case CHECK(
				local = LOWER(local)
			),
			ADD CONSTRAINT user_unverified_email_fk_domain_id
				FOREIGN KEY(domain_id) REFERENCES personal_domain(id),
			ADD CONSTRAINT user_unverified_email_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id);
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
		REFERENCES user_email(
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
