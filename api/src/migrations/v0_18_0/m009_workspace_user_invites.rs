//! Adds the `workspace_user_invite` and `workspace_user_invite_role` tables
//! that back the email-based "invite a user to a workspace" flow. An invite
//! stores the invited email plus an argon2 hash of the emailed token and its
//! expiry, and the roles the invitee will be granted are held in the companion
//! table until they accept.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE workspace_user_invite(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			email TEXT NOT NULL,

			token_hash TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,

			invited_by UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE workspace_user_invite_role(
			invite_id UUID NOT NULL,
			role_id UUID NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite
		ADD CONSTRAINT workspace_user_invite_pk
		PRIMARY KEY(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			workspace_user_invite_uq_workspace_id_email
		ON
			workspace_user_invite(workspace_id, email);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			workspace_user_invite_idx_token_expiry
		ON
			workspace_user_invite(token_expiry);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_pk
		PRIMARY KEY(invite_id, role_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite
			ADD CONSTRAINT workspace_user_invite_chk_email_is_lower_case CHECK(
				email = LOWER(email)
			),
			ADD CONSTRAINT workspace_user_invite_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_user_invite_fk_invited_by
				FOREIGN KEY(invited_by) REFERENCES "user"(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
			ADD CONSTRAINT workspace_user_invite_role_fk_invite_id
				FOREIGN KEY(invite_id) REFERENCES workspace_user_invite(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
