use std::collections::BTreeMap;

use models::rbac::WorkspacePermission;

use crate::prelude::*;

/// Compute the permission map for a service account.
///
/// A service account is its own actor, so there is no membership row to go
/// through: its bindings hang directly off its id. It belongs to exactly one
/// workspace and is never that workspace's super admin —
/// `workspace.super_admin_id` only ever points at a user — so the map has at
/// most one entry and it is always a [`WorkspacePermission::Member`].
///
/// A pure database read; caching is the dispatcher's job.
#[tracing::instrument(skip(db_connection))]
pub async fn get_permissions_for_service_account(
	db_connection: &mut DatabaseConnection,
	service_account_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	let mut service_account_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	// Membership is first-class: an account holding no bindings still belongs
	// to its workspace, and gets an entry with an empty permission map.
	let Some(workspace) = query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!: Uuid"
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		service_account_id as _,
	)
	.fetch_optional(&mut *db_connection)
	.await?
	else {
		return Ok(service_account_permissions);
	};

	service_account_permissions.insert(
		workspace.workspace_id,
		WorkspacePermission::Member {
			permissions: BTreeMap::new(),
		},
	);

	// One query over bindings: a workspace-scope row (scope_id =
	// workspace_id) grants a permission everywhere in the workspace;
	// resource-scope rows accumulate into a resource set.
	query!(
		r#"
		SELECT
			role_binding.workspace_id AS "workspace_id!",
			role_permission.permission_id AS "permission_id!",
			role_binding.scope_id AS "scope_id!"
		FROM
			role_binding
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = role_binding.role_id
		WHERE
			role_binding.actor_id = $1;
		"#,
		service_account_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = service_account_permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let WorkspacePermission::Member { permissions } = permissions else {
			error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			return;
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	});

	Ok(service_account_permissions)
}
