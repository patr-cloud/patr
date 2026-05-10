//! Collapses the user identity model to a single email.
//!
//! Renames `user.username` (VARCHAR(100)) to `user.email` (TEXT) so the
//! column position is preserved, backfills it from `recovery_email`, and
//! drops `user.recovery_email`, `user.recovery_phone_*`, plus the
//! `user_email`, `user_unverified_email`, `user_phone_number`,
//! `user_unverified_phone_number` and `phone_number_country_code` tables.
//! Rebuilds `user_to_sign_up` keyed by email instead of username.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Drop dependent satellite tables first.
	for stmt in [
		"DROP TABLE IF EXISTS user_unverified_phone_number CASCADE",
		"DROP TABLE IF EXISTS user_phone_number CASCADE",
		"DROP TABLE IF EXISTS phone_number_country_code CASCADE",
		"DROP TABLE IF EXISTS user_unverified_email CASCADE",
		"DROP TABLE IF EXISTS user_email CASCADE",
	] {
		sqlx::query(stmt).execute(&mut *connection).await?;
	}

	// Drop user constraints that depend on columns we're about to drop or
	// rename. CASCADE is safer than enumerating each constraint name.
	sqlx::query(
		r#"
		ALTER TABLE "user"
			DROP CONSTRAINT IF EXISTS user_chk_email_or_phone_present,
			DROP CONSTRAINT IF EXISTS user_chk_recovery_phone_country_code_is_upper_case,
			DROP CONSTRAINT IF EXISTS user_chk_username_is_valid,
			DROP CONSTRAINT IF EXISTS user_uq_recovery_phone_country_code_recovery_phone_number,
			DROP CONSTRAINT IF EXISTS user_uq_recovery_email,
			DROP CONSTRAINT IF EXISTS user_uq_username;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Backfill any NULL recovery_email so the NOT NULL email column is safe.
	sqlx::query(
		r#"
		UPDATE "user"
		SET recovery_email = LOWER(id::text || '@placeholder.invalid')
		WHERE recovery_email IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Promote `username` → `email` in place to preserve column order.
	// Widen VARCHAR(100) to TEXT, copy recovery_email into it, then rename.
	sqlx::query(r#"ALTER TABLE "user" ALTER COLUMN username TYPE TEXT;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"UPDATE "user" SET username = recovery_email;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE "user" RENAME COLUMN username TO email;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		ALTER TABLE "user"
			DROP COLUMN IF EXISTS recovery_email,
			DROP COLUMN IF EXISTS recovery_phone_country_code,
			DROP COLUMN IF EXISTS recovery_phone_number;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"ALTER TABLE "user" ALTER COLUMN email SET NOT NULL;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE "user" ADD CONSTRAINT user_uq_email UNIQUE(email);"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		ALTER TABLE "user"
			ADD CONSTRAINT user_chk_email_is_lower_case CHECK(email = LOWER(email));
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Rebuild user_to_sign_up keyed by email instead of username.
	sqlx::query("DROP TABLE IF EXISTS user_to_sign_up CASCADE;")
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		CREATE TABLE user_to_sign_up(
			email TEXT NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_to_sign_up
			ADD CONSTRAINT user_to_sign_up_pk PRIMARY KEY(email),
			ADD CONSTRAINT user_to_sign_up_chk_email_is_lower_case
				CHECK(email = LOWER(email));
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query("CREATE INDEX user_to_sign_up_idx_otp_expiry ON user_to_sign_up(otp_expiry);")
		.execute(&mut *connection)
		.await?;

	Ok(())
}
