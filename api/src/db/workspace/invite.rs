use crate::prelude::*;

/// Initializes the workspace invite tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_user_invite_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace invite tables");
	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The roles the invitee will be granted once they accept. Mirrors the
	// multi-row role model of `workspace_user`.
	query!(
		r#"
		CREATE TABLE workspace_user_invite_role(
			invite_id UUID NOT NULL,
			role_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the workspace invite indices
#[instrument(skip(connection))]
pub async fn initialize_workspace_user_invite_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace invite indices");
	query!(
		r#"
		ALTER TABLE workspace_user_invite
		ADD CONSTRAINT workspace_user_invite_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// One pending invite per email per workspace. Re-inviting the same email
	// upserts on this index rather than creating a duplicate.
	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_user_invite_uq_workspace_id_email
		ON
			workspace_user_invite(workspace_id, email);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_user_invite_idx_token_expiry
		ON
			workspace_user_invite(token_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_pk
		PRIMARY KEY(invite_id, role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the workspace invite constraints
#[instrument(skip(connection))]
pub async fn initialize_workspace_user_invite_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace invite constraints");
	query!(
		r#"
		ALTER TABLE workspace_user_invite
			ADD CONSTRAINT workspace_user_invite_chk_email_is_lower_case CHECK(
				email = LOWER(email)
			),
			ADD CONSTRAINT workspace_user_invite_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_user_invite_fk_invited_by
				FOREIGN KEY(invited_by) REFERENCES "user"(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// `role_id` is intentionally left without a foreign key: invites are
	// staging rows, and a role may be deleted while an invite still references
	// it. The accept handler re-validates every role against the workspace
	// before granting it, so stale role ids are simply filtered out then.
	query!(
		r#"
		ALTER TABLE workspace_user_invite_role
			ADD CONSTRAINT workspace_user_invite_role_fk_invite_id
				FOREIGN KEY(invite_id) REFERENCES workspace_user_invite(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
