use std::collections::{BTreeMap, BTreeSet};

use models::rbac::{ResourcePermissionType, WorkspacePermission};

use crate::prelude::*;

/// Compute the permission map for a service account.
///
/// Unlike a user API token there is nothing to intersect: a service account
/// holds no scope of its own, so its permissions are exactly the ones granted
/// by the roles assigned to it in `service_account_role`. A service account is
/// also never a workspace super-admin — `workspace.super_admin_id` only ever
/// points at a user — so there is no super-admin pass here.
///
/// The workspace each permission lands in comes from the service account's own
/// `workspace_id`, not from the role, which keeps a role in another workspace
/// from leaking permissions across the boundary.
#[tracing::instrument(skip(db_connection))]
pub async fn get_permissions_for_service_account(
	db_connection: &mut DatabaseConnection,
	service_account_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	let mut service_account_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			service_account.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_exclude.resource_id AS "resource_id?"
		FROM
			service_account_role
		INNER JOIN
			service_account
		ON
			service_account.id = service_account_role.service_account_id
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = service_account_role.role_id AND
			role_resource_permissions_type.permission_type = 'exclude'
		LEFT JOIN
			role_resource_permissions_exclude
		ON
			role_resource_permissions_exclude.role_id = service_account_role.role_id
		WHERE
			service_account_role.service_account_id = $1;
		"#,
		service_account_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = service_account_permissions
			.entry(workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		match permissions {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				let permission_type = permissions
					.entry(permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Exclude(BTreeSet::new()));
				match permission_type {
					ResourcePermissionType::Include(_) => {
						error!(
							"Found include permissions before include is even called. This should be possible!"
						);
					}
					ResourcePermissionType::Exclude(resources) => {
						let Some(resource_id) = resource_id else {
							return;
						};

						resources.insert(resource_id.into());
					}
				}
			}
		}
	});

	query!(
		r#"
		SELECT
			service_account.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_include.resource_id AS "resource_id?"
		FROM
			service_account_role
		INNER JOIN
			service_account
		ON
			service_account.id = service_account_role.service_account_id
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = service_account_role.role_id AND
			role_resource_permissions_type.permission_type = 'include'
		LEFT JOIN
			role_resource_permissions_include
		ON
			role_resource_permissions_include.role_id = service_account_role.role_id
		WHERE
			service_account_role.service_account_id = $1;
		"#,
		service_account_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = service_account_permissions
			.entry(workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let Some(resource_id) = resource_id else {
			return;
		};

		match permissions {
			WorkspacePermission::SuperAdmin => {
				error!("SuperAdmin found when Member expected. This shouldn't be possible!");
			}
			WorkspacePermission::Member { permissions } => {
				let permission_type = permissions
					.entry(permission_id.into())
					.or_insert_with(|| ResourcePermissionType::Include(BTreeSet::new()));
				match permission_type {
					ResourcePermissionType::Include(resources) => {
						resources.insert(resource_id.into());
					}
					ResourcePermissionType::Exclude(resources) => {
						resources.remove(&resource_id.into());
					}
				}
			}
		}
	});

	Ok(service_account_permissions)
}
