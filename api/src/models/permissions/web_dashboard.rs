use std::collections::BTreeMap;

use models::rbac::WorkspacePermission;
use time::OffsetDateTime;

use crate::{
	models::redis::{ActorAuthDataCache, ActorAuthDataCacheKind},
	prelude::*,
};

/// Load everything the cache holds for the web login `login_id`: the user
/// behind it and their permissions. A login that no longer exists (logged out
/// or deleted) has no row to find and is rejected here.
pub(super) async fn load_actor_auth_data(
	database: &mut DatabaseConnection,
	login_id: &Uuid,
) -> Result<ActorAuthDataCache, ErrorType> {
	// Taken before the lookup, so a stamp written while the lookup is in
	// flight still marks this entry stale.
	let created_at = OffsetDateTime::now_utc();

	let Some(user) = query! {
		r#"
		SELECT
			"user".*
		FROM
			"user"
		INNER JOIN
			user_login
		ON
			"user".id = user_login.user_id
		INNER JOIN
			web_login
		ON
			user_login.login_id = web_login.login_id
		WHERE
			user_login.login_id = $1 AND
			user_login.login_type = 'web_login';
		"#,
		login_id as _
	}
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("web login not found");
		// No specific error for the login not being found, since we don't want
		// to leak information about whether a loginId is valid or if it's
		// expired
		return Err(ErrorType::AuthorizationTokenInvalid);
	};
	trace!("Web login exists in the database");

	// Note: `web_login.token_expiry` is the refresh token's lifetime, not the
	// access token's. Access token validity is gated by the JWT's own `exp`
	// claim, checked on every request. Re-checking `token_expiry` here would prevent
	// a fresh JWT (post-refresh) from authenticating until the entire session
	// is renewed, and would also keep an old, expired JWT alive as long as
	// the session itself was still fresh. Both are wrong.

	let mut permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			id AS "workspace_id!"
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		user.id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	// Membership is first-class: a member holding no roles still belongs to
	// the workspace, and gets an entry with an empty permission map.
	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!: Uuid"
		FROM
			workspace_user
		WHERE
			user_id = $1;
		"#,
		user.id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		permissions
			.entry(workspace_id)
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});
	});

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
			workspace_user
		INNER JOIN
			role_binding
		ON
			role_binding.actor_id = workspace_user.actor_id
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = role_binding.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user.id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let WorkspacePermission::Member { permissions } = permissions else {
			// Super admin of this workspace — bindings are redundant.
			return;
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	});

	Ok(ActorAuthDataCache {
		actor_id: user.id.into(),
		kind: ActorAuthDataCacheKind::WebLogin {
			email: user.email,
			first_name: user.first_name,
			last_name: user.last_name,
			created: user.created,
		},
		permissions,
		created_at,
	})
}
