//! Adds a `sign_up_attempts` counter to `user_to_sign_up` so the
//! OTP-verification step in `complete_sign_up` can gate brute-force attempts
//! the same way `reset_password` already does for password resets. The
//! counter is cumulative across the OTP's lifetime — there's no per-request
//! reset path, which is intentional (a slow-drip attacker who waits for OTP
//! expiry between batches still hits the same hard ceiling).

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Add the column with a temporary default so the existing rows get a
	// concrete value, then drop the default so future INSERTs must set it
	// explicitly. New sign-ups go through `create_account` which now sets it
	// to 0; the UPSERT branch leaves it untouched so the counter accumulates
	// across re-requests within the OTP lifetime.
	sqlx::query(
		r#"
		ALTER TABLE user_to_sign_up
			ADD COLUMN sign_up_attempts INTEGER NOT NULL DEFAULT 0;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_to_sign_up
			ALTER COLUMN sign_up_attempts DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
