//! Adds the `user_social_login` table for OAuth SSO support.
//!
//! Links a Patr user account to a third-party OAuth identity. The composite
//! primary key `(provider, external_id)` is stable across providers. A UNIQUE
//! constraint on `(user_id, provider)` prevents linking more than one account
//! per OAuth provider per Patr user.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE SOCIAL_LOGIN_PROVIDER AS ENUM(
			'github'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE user_social_login(
			user_id     UUID                  NOT NULL,
			provider    SOCIAL_LOGIN_PROVIDER NOT NULL,
			external_id TEXT                  NOT NULL,
			linked_at   TIMESTAMPTZ           NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_social_login
			ADD CONSTRAINT user_social_login_pk
				PRIMARY KEY (provider, external_id),
			ADD CONSTRAINT user_social_login_uq_user_provider
				UNIQUE (user_id, provider),
			ADD CONSTRAINT user_social_login_fk_user_id
				FOREIGN KEY (user_id) REFERENCES "user"(id),
			ADD CONSTRAINT user_social_login_chk_external_id_not_empty
				CHECK (external_id <> '');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX user_social_login_idx_user_id
			ON user_social_login(user_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
