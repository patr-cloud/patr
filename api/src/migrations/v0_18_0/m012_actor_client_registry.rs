//! Introduces `actor_client` — the registry of credentials that can act and
//! leave an audit trail.
//!
//! Today every actor is a user acting through a `user_login` (web session or
//! API token), so the audit log's `login_id` answers "who did this". Service
//! accounts and OAuth apps will act through credentials that are *not* user
//! logins, and the audit log must not grow a column per credential kind.
//! `actor_client` is the supertype: `user_login` registers into it (reusing
//! `login_id` as the registry id, the same trick `resource` uses), and future
//! credential kinds register alongside without touching `audit_log` again.
//!
//! The generated `client_type` discriminator plus the composite FK pin every
//! `user_login` row to a registry row of the right kind — the same pattern as
//! `web_login`/`user_api_token` pointing up at `user_login` itself.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE ACTOR_CLIENT_TYPE AS ENUM(
			'user_login'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE actor_client(
			id UUID NOT NULL,
			client_type ACTOR_CLIENT_TYPE NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor_client
			ADD CONSTRAINT actor_client_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_client_uq_id_client_type UNIQUE(id, client_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Register every existing login, reusing login_id as the registry id so
	// audit_log's stored values are already valid actor_client ids.
	sqlx::query(
		r#"
		INSERT INTO
			actor_client(id, client_type)
		SELECT
			login_id,
			'user_login'
		FROM
			user_login;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ADD COLUMN client_type ACTOR_CLIENT_TYPE NOT NULL
			GENERATED ALWAYS AS ('user_login') STORED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ADD CONSTRAINT user_login_fk_login_id_client_type
		FOREIGN KEY(login_id, client_type) REFERENCES actor_client(id, client_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
