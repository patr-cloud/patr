use std::collections::BTreeMap;

use models::rbac::WorkspacePermission;
use time::{Duration, OffsetDateTime};

use crate::{
	models::redis::{ActorAuthDataCache, ActorAuthDataCacheKind},
	prelude::*,
};

/// Load everything the cache holds for the service account
/// `service_account_id`: the account itself and its permissions. A service
/// account holds a single, non-rotating credential rather than a set of
/// logins, so it is its own login ID and its own actor ID.
///
/// A service account is its own actor, so there is no membership row to go
/// through: its bindings hang directly off its id. It belongs to exactly one
/// workspace and is never that workspace's super admin —
/// `workspace.super_admin_id` only ever points at a user — so the permission
/// map has exactly one entry and it is always a
/// [`WorkspacePermission::Member`].
pub(super) async fn load_actor_auth_data(
	database: &mut DatabaseConnection,
	service_account_id: &Uuid,
) -> Result<(ActorAuthDataCache, Duration), ErrorType> {
	// Taken before the lookup, so a stamp written while the lookup is in
	// flight still marks this entry stale.
	let created_at = OffsetDateTime::now_utc();

	let Some(service_account) = query!(
		r#"
		SELECT
			workspace_id AS "workspace_id: Uuid",
			name,
			token_hash,
			created
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		service_account_id as _,
	)
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("The service account has been deleted");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	// Membership is first-class: an account holding no bindings still belongs
	// to its workspace, and gets an entry with an empty permission map.
	let mut permissions = BTreeMap::from([(
		service_account.workspace_id,
		WorkspacePermission::Member {
			permissions: BTreeMap::new(),
		},
	)]);

	// One query over bindings: a workspace-scope row (scope_id =
	// workspace_id) grants a permission everywhere in the workspace;
	// resource-scope rows accumulate into a resource set.
	for row in query!(
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
	.fetch_all(&mut *database)
	.await?
	{
		// The `workspace_actor` FK pins every binding to the account's own
		// workspace, so a binding elsewhere means the data is corrupt. Don't
		// widen the account's access to match.
		let Some(WorkspacePermission::Member {
			permissions: workspace_permissions,
		}) = permissions.get_mut(&row.workspace_id.into())
		else {
			error!(
				concat!(
					"Service account `{}` has a role binding in ",
					"workspace `{}` outside its own workspace `{}`"
				),
				service_account_id, row.workspace_id, service_account.workspace_id
			);
			return Err(ErrorType::server_error(
				"service account bound outside its workspace",
			));
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		workspace_permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	}

	Ok((
		ActorAuthDataCache {
			actor_id: *service_account_id,
			kind: ActorAuthDataCacheKind::ServiceAccount {
				name: service_account.name,
				created: service_account.created,
				token_hash: service_account.token_hash,
			},
			permissions,
			created_at,
		},
		constants::CACHED_PERMISSIONS_VALIDITY,
	))
}
