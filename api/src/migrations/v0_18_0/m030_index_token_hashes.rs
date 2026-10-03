//! Index `token_hash` on API tokens and service accounts, which are now looked
//! up by the SHA-256 of the token rather than by a login ID carried in it.
//!
//! No data changes. Existing rows keep their argon2 hashes, which no SHA-256
//! can match, so tokens issued before this change stop authenticating until
//! they're regenerated. They're kept rather than cleared so they can still be
//! upgraded on first use.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			user_api_token_uq_token_hash
		ON
			user_api_token(token_hash);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			service_account_uq_token_hash
		ON
			service_account(token_hash);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
