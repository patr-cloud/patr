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
			email CITEXT NOT NULL,

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
			workspace_id UUID NOT NULL,
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
				PRIMARY KEY(id),
			ADD CONSTRAINT workspace_user_invite_uq_id_workspace_id
				UNIQUE(id, workspace_id);
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
			ADD CONSTRAINT workspace_user_invite_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_user_invite_fk_invited_by
				FOREIGN KEY(invited_by) REFERENCES "user"(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Both keys carry `workspace_id`, so the database itself guarantees an
	// invite can only ever grant roles from the workspace it was sent for —
	// there is no pair of rows that says otherwise. `delete_role` clears these
	// rows alongside the `workspace_user` ones when a role goes away.
	query!(
		r#"
		ALTER TABLE workspace_user_invite_role
			ADD CONSTRAINT workspace_user_invite_role_fk_invite_id_workspace_id
				FOREIGN KEY(invite_id, workspace_id)
					REFERENCES workspace_user_invite(id, workspace_id),
			ADD CONSTRAINT workspace_user_invite_role_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
