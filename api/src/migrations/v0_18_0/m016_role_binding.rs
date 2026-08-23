//! Adds `role_binding` and `api_token_role_binding`.
//!
//! `role_binding(actor, role, scope)` is the only place a permission target
//! appears: the scope is a `resource` id, and a workspace-wide grant is
//! simply `scope_id = workspace_id` (workspaces self-own their resource
//! row), so no scope-type column is needed. Every FK pivots on
//! `workspace_id`, making cross-workspace grants unrepresentable: the actor,
//! the role, and the scope must all live in the binding's workspace.
//!
//! `api_token_role_binding` holds a token's own (role, scope) rows —
//! independent of the owner's bindings, not references to them. They are a
//! ceiling: effective permissions are the ceiling intersected with the
//! owner's current permissions, computed at auth time. A ceiling above the
//! owner's reach is allowed and clamps harmlessly.
//!
//! Deliberately no `deleted` column in the scope FKs (unlike the legacy
//! token permission tables, where the generated-NULL trick makes
//! soft-deleting a referenced resource fail): a binding onto a tombstoned
//! resource is inert, because the authorizer re-checks `deleted IS NULL`.
//!
//! Additive: nothing reads or writes these until the backfill and cutover.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE role_binding(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_id UUID NOT NULL,
			role_id UUID NOT NULL,
			scope_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			created_by UUID
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE api_token_role_binding(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL,
			scope_id UUID NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE role_binding
			ADD CONSTRAINT role_binding_pk PRIMARY KEY(id),
			ADD CONSTRAINT role_binding_uq_actor_id_role_id_scope_id
				UNIQUE(actor_id, role_id, scope_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			role_binding_idx_actor_id
		ON
			role_binding
		(actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			role_binding_idx_scope_id
		ON
			role_binding
		(scope_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE api_token_role_binding
		ADD CONSTRAINT api_token_role_binding_pk
		PRIMARY KEY(token_id, role_id, scope_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE role_binding
			ADD CONSTRAINT role_binding_fk_actor_id_workspace_id
				FOREIGN KEY(actor_id, workspace_id) REFERENCES actor(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_scope_id_workspace_id
				FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_created_by
				FOREIGN KEY(created_by) REFERENCES "user"(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE api_token_role_binding
			ADD CONSTRAINT api_token_role_binding_fk_token_id
				FOREIGN KEY(token_id) REFERENCES user_api_token(token_id),
			ADD CONSTRAINT api_token_role_binding_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id),
			ADD CONSTRAINT api_token_role_binding_fk_scope_id_workspace_id
				FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
