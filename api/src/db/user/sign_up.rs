use crate::prelude::*;

/// Initializes the user sign up tables
#[instrument(skip(connection))]
pub async fn initialize_user_sign_up_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user sign up tables");
	query!(
		r#"
		CREATE TABLE user_to_sign_up(
			email TEXT NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,

			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user sign up indices
#[instrument(skip(connection))]
pub async fn initialize_user_sign_up_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user sign up indices");
	query!(
		r#"
		ALTER TABLE user_to_sign_up
		ADD CONSTRAINT user_to_sign_up_pk
		PRIMARY KEY(email);
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

	Ok(())
}

/// Initializes the user sign up constraints
#[instrument(skip(connection))]
pub async fn initialize_user_sign_up_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user sign up constraints");
	query!(
		r#"
		ALTER TABLE user_to_sign_up
			ADD CONSTRAINT user_to_sign_up_chk_email_is_lower_case CHECK(
				email = LOWER(email)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
