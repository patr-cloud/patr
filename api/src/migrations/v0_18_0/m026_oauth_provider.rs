//! Adds the OAuth 2.1 / OpenID Connect provider schema: clients, logins and
//! rotating refresh tokens.
//!
//! `USER_LOGIN_TYPE` is swapped rather than extended. `ALTER TYPE ... ADD
//! VALUE` works inside a transaction, but the new value can't be used until
//! it commits, and `oauth_login.login_type` is a generated column that names
//! it:
//!
//! ```text
//! ERROR:  unsafe use of new value "oauth_login" of enum type USER_LOGIN_TYPE
//! HINT:   New enum values must be committed before they can be used.
//! ```
//!
//! All migrations run in one transaction, so a second file wouldn't help. A
//! type *created* in the transaction has no such restriction, so the old one
//! is renamed aside, a replacement created with all three values, the columns
//! repointed through a text round-trip, and the original dropped — the same
//! move as `m013_drop_static_site_url_type`.
//!
//! `web_login.login_type` and `user_api_token.login_type` are STORED generated
//! columns of this type. Postgres refuses `USING` on a generated column, so
//! those are retyped through a temporary implicit cast and their expressions
//! rewritten with `SET EXPRESSION` — in place, so the column order matches a
//! fresh database. Both tables are rewritten under `ACCESS EXCLUSIVE`; seconds,
//! at one row per session or token.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	swap_user_login_type(connection).await?;
	create_oauth_client(connection).await?;
	create_oauth_login(connection).await?;
	create_oauth_refresh_token(connection).await?;

	Ok(())
}

/// Replaces `USER_LOGIN_TYPE` with one that also carries `'oauth_login'`,
/// retyping every column that uses it in place.
async fn swap_user_login_type(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Both sides of a foreign key must share a type, so the composite FKs and
	// the unique they reference come off while the columns change underneath.
	sqlx::query(
		r#"
		ALTER TABLE web_login
		DROP CONSTRAINT web_login_fk;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
		DROP CONSTRAINT user_api_token_token_id_user_id_login_type_fk;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		DROP CONSTRAINT user_login_uq_login_id_user_id_login_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TYPE USER_LOGIN_TYPE RENAME TO USER_LOGIN_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE USER_LOGIN_TYPE AS ENUM(
			'api_token',
			'oauth_login',
			'web_login'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN login_type TYPE USER_LOGIN_TYPE
			USING login_type::TEXT::USER_LOGIN_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// A generated column can't take a `USING`, but it can be retyped when an
	// implicit cast exists. Its expression still names the old type after
	// that, so it's rewritten too.
	sqlx::query(
		r#"
		CREATE CAST (USER_LOGIN_TYPE_OLD AS USER_LOGIN_TYPE)
			WITH INOUT AS IMPLICIT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
		ALTER COLUMN login_type TYPE USER_LOGIN_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
		ALTER COLUMN login_type SET EXPRESSION AS ('web_login');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
		ALTER COLUMN login_type TYPE USER_LOGIN_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
		ALTER COLUMN login_type SET EXPRESSION AS ('api_token');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP CAST (USER_LOGIN_TYPE_OLD AS USER_LOGIN_TYPE);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE USER_LOGIN_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ADD CONSTRAINT user_login_uq_login_id_user_id_login_type UNIQUE(
			login_id,
			user_id,
			login_type
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
		ADD CONSTRAINT web_login_fk
		FOREIGN KEY(
			login_id,
			user_id,
			login_type
		) REFERENCES user_login(
			login_id,
			user_id,
			login_type
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
		ADD CONSTRAINT user_api_token_token_id_user_id_login_type_fk
		FOREIGN KEY(
			token_id,
			user_id,
			login_type
		) REFERENCES user_login(
			login_id,
			user_id,
			login_type
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Creates `oauth_client`, the boot-time mirror of the clients declared in
/// the config.
async fn create_oauth_client(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE oauth_client(
			client_id TEXT NOT NULL,
			name TEXT NOT NULL,
			logo_url TEXT NOT NULL,
			client_uri TEXT NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE oauth_client
			ADD CONSTRAINT oauth_client_pk PRIMARY KEY(client_id),
			ADD CONSTRAINT oauth_client_chk_client_id_not_empty CHECK(client_id != '');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Creates `oauth_login`, the grant itself.
async fn create_oauth_login(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE oauth_login(
			login_id UUID NOT NULL,
			user_id UUID NOT NULL,
			client_id TEXT NOT NULL,
			scope TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			last_used TIMESTAMPTZ NOT NULL,
			revoked TIMESTAMPTZ,
			created_ip INET NOT NULL,
			created_user_agent TEXT NOT NULL,
			login_type USER_LOGIN_TYPE NOT NULL
				GENERATED ALWAYS AS ('oauth_login') STORED
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE oauth_login
			ADD CONSTRAINT oauth_login_pk PRIMARY KEY(login_id),
			ADD CONSTRAINT oauth_login_uq_login_id_user_id UNIQUE(login_id, user_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			oauth_login_idx_user_id_client_id
		ON
			oauth_login(user_id, client_id)
		WHERE
			revoked IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE oauth_login
			ADD CONSTRAINT oauth_login_fk
				FOREIGN KEY(
					login_id,
					user_id,
					login_type
				) REFERENCES user_login(
					login_id,
					user_id,
					login_type
				),
			ADD CONSTRAINT oauth_login_fk_client_id
				FOREIGN KEY(client_id) REFERENCES oauth_client(client_id),
			ADD CONSTRAINT oauth_login_chk_scope_not_empty CHECK(scope != '');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Creates `oauth_refresh_token`, the rotating refresh tokens for a grant.
async fn create_oauth_refresh_token(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE oauth_refresh_token(
			id UUID NOT NULL,
			login_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			expiry TIMESTAMPTZ NOT NULL,
			consumed TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE oauth_refresh_token
			ADD CONSTRAINT oauth_refresh_token_pk PRIMARY KEY(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			oauth_refresh_token_uq_login_id
		ON
			oauth_refresh_token(login_id)
		WHERE
			consumed IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			oauth_refresh_token_idx_login_id
		ON
			oauth_refresh_token(login_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE oauth_refresh_token
			ADD CONSTRAINT oauth_refresh_token_fk_login_id
				FOREIGN KEY(login_id) REFERENCES oauth_login(login_id),
			ADD CONSTRAINT oauth_refresh_token_chk_created_before_expiry
				CHECK(created < expiry);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
