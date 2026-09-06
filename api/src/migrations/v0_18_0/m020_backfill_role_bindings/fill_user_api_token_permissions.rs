use crate::prelude::*;

/// Expands a token's declared per-permission resource lists into
/// `(permission, scope)` ceiling rows, by the same three rules as
/// [`super::fill_role_bindings`].
///
/// The legacy token tables are already keyed on `(token, workspace,
/// permission)`, so this rewrites the same grants in the new shape — nothing
/// widens, nothing narrows, and no role is involved.
pub(super) async fn fill_user_api_token_permissions(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	// Exclude(∅): the whole workspace, one row at scope = workspace.
	sqlx::query(
		r#"
		INSERT INTO
			user_api_token_permission_binding(
				token_id,
				workspace_id,
				permission_id,
				scope_id
			)
		SELECT
			d.token_id,
			d.workspace_id,
			d.permission_id,
			d.workspace_id
		FROM
			user_api_token_resource_permissions_type d
		WHERE
			d.resource_permission_type = 'exclude' AND
			NOT EXISTS (
				SELECT
					1
				FROM
					user_api_token_resource_permissions_exclude e
				WHERE
					e.token_id = d.token_id AND
					e.workspace_id = d.workspace_id AND
					e.permission_id = d.permission_id
			)
		ON CONFLICT
			(token_id, permission_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Include(S): one row per live, same-workspace member of S.
	sqlx::query(
		r#"
		INSERT INTO
			user_api_token_permission_binding(
				token_id,
				workspace_id,
				permission_id,
				scope_id
			)
		SELECT
			d.token_id,
			d.workspace_id,
			d.permission_id,
			i.resource_id
		FROM
			user_api_token_resource_permissions_type d
		INNER JOIN
			user_api_token_resource_permissions_include i
		ON
			i.token_id = d.token_id AND
			i.workspace_id = d.workspace_id AND
			i.permission_id = d.permission_id
		INNER JOIN
			resource r
		ON
			r.id = i.resource_id AND
			r.workspace_id = d.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		WHERE
			d.resource_permission_type = 'include'
		ON CONFLICT
			(token_id, permission_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Exclude(S≠∅): one row per live workspace resource not in S.
	sqlx::query(
		r#"
		INSERT INTO
			user_api_token_permission_binding(
				token_id,
				workspace_id,
				permission_id,
				scope_id
			)
		SELECT
			d.token_id,
			d.workspace_id,
			d.permission_id,
			r.id
		FROM
			user_api_token_resource_permissions_type d
		INNER JOIN
			resource r
		ON
			r.workspace_id = d.workspace_id AND
			r.deleted IS NULL AND
			r.id <> r.workspace_id
		WHERE
			d.resource_permission_type = 'exclude' AND
			EXISTS (
				SELECT
					1
				FROM
					user_api_token_resource_permissions_exclude e
				WHERE
					e.token_id = d.token_id AND
					e.workspace_id = d.workspace_id AND
					e.permission_id = d.permission_id
			) AND NOT EXISTS (
				SELECT
					1
				FROM
					user_api_token_resource_permissions_exclude e2
				WHERE
					e2.token_id = d.token_id AND
					e2.workspace_id = d.workspace_id AND
					e2.permission_id = d.permission_id AND
					e2.resource_id = r.id
			)
		ON CONFLICT
			(token_id, permission_id, scope_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
