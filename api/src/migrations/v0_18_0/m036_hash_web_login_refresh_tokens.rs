//! Stores web login refresh tokens as the SHA-256 of a `patr_rf_…` token in
//! `web_login.token_hash`, looked up by it, instead of argon2 hashes of a
//! secret found through the login ID carried in the token.
//!
//! No argon2 hash can match a SHA-256, so every existing session is deleted,
//! as m035 does, along with its `user_login` row. The `actor_client` rows stay
//! for the audit log.
//!
//! The NOT NULL constraint keeps the name it was created with, so it's renamed
//! with the column.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM web_login;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_type = 'web_login';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN refresh_token TO token_hash;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
		RENAME CONSTRAINT web_login_refresh_token_not_null TO web_login_token_hash_not_null;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			web_login_uq_token_hash
		ON
			web_login(token_hash);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
