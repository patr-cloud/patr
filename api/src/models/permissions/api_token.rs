use std::collections::BTreeMap;

use models::rbac::{WorkspacePermission, intersect_workspace_permissions};
use sqlx::Connection as _;
use time::{Duration, OffsetDateTime};

use crate::{
	models::redis::{ActorAuthDataCache, ActorAuthDataCacheKind},
	prelude::*,
};

/// Load everything the cache holds for the API token whose hash is
/// `token_hash`: the user behind it, the token's restrictions, and its
/// effective permissions. Also says how long the entry may live: until the
/// token expires, at most [`constants::CACHED_PERMISSIONS_VALIDITY`] — expiry
/// isn't kept in the entry; the entry just doesn't outlive the token.
pub(super) async fn load_actor_auth_data(
	database: &mut DatabaseConnection,
	token_hash: &str,
) -> Result<(ActorAuthDataCache, Duration), ErrorType> {
	// Taken before the lookup, so a stamp written while the lookup is in
	// flight still marks this entry stale.
	let created_at = OffsetDateTime::now_utc();
	let now = created_at;

	// Waits out an uncommitted revoke, update or regenerate of the row, then
	// releases the lock right away so a long request doesn't hold it.
	let mut savepoint = database.begin().await?;
	let token = query!(
		r#"
		SELECT
			user_api_token.token_id AS "token_id: Uuid",
			user_api_token.user_id AS "user_id: Uuid",
			user_api_token.token_nbf,
			user_api_token.token_exp,
			user_api_token.allowed_ips,
			user_api_token.revoked,
			"user".email,
			"user".first_name,
			"user".last_name,
			"user".created
		FROM
			user_api_token
		INNER JOIN
			"user"
		ON
			"user".id = user_api_token.user_id
		WHERE
			user_api_token.token_hash = $1
		FOR SHARE OF
			user_api_token;
		"#,
		token_hash,
	)
	.fetch_optional(&mut *savepoint)
	.await?;
	savepoint.rollback().await?;

	let Some(token) = token else {
		// No specific error for the token not being found, since we don't
		// want to leak whether a token ever existed
		warn!("No API token with this hash");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	if token.revoked.is_some() {
		info!("API token has been revoked");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}

	if let Some(nbf) = token.token_nbf &&
		now < nbf
	{
		info!("API token is not valid yet");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}

	let ttl = match token.token_exp {
		Some(exp) if now > exp => {
			info!("API token has expired");
			return Err(ErrorType::AuthorizationTokenInvalid);
		}
		Some(exp) => constants::CACHED_PERMISSIONS_VALIDITY.min(exp - now),
		None => constants::CACHED_PERMISSIONS_VALIDITY,
	};

	// User's current role-derived permissions (the upper bound for the
	// token).
	let mut user_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			id AS "workspace_id!"
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		token.user_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		user_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

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
		token.user_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = user_permissions
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

	// Token's declared permissions (the snapshot at mint/patch time).
	let mut token_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!"
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token.token_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		token_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	// The token's declared ceiling: its own (permission, scope) rows.
	query!(
		r#"
		SELECT
			user_api_token_permission_binding.workspace_id AS "workspace_id",
			user_api_token_permission_binding.permission_id AS "permission_id",
			user_api_token_permission_binding.scope_id AS "scope_id"
		FROM
			user_api_token_permission_binding
		WHERE
			user_api_token_permission_binding.token_id = $1;
		"#,
		token.token_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.for_each(|row| {
		let permissions = token_permissions
			.entry(row.workspace_id.into())
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});

		let WorkspacePermission::Member { permissions } = permissions else {
			// Super admin of this workspace — declared rows are redundant.
			return;
		};

		// A scope is just a resource id; the workspace's own id is the root
		// and covers everything under it.
		permissions
			.entry(row.permission_id.into())
			.or_default()
			.insert(row.scope_id.into());
	});

	let permissions = intersect_workspace_permissions(&token_permissions, &user_permissions);

	Ok((
		ActorAuthDataCache {
			login_id: token.token_id,
			actor_id: token.user_id,
			kind: ActorAuthDataCacheKind::ApiToken {
				email: token.email,
				first_name: token.first_name,
				last_name: token.last_name,
				created: token.created,
				allowed_ips: token.allowed_ips,
			},
			permissions,
			created_at,
		},
		ttl,
	))
}
