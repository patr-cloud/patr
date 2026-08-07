use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	str::FromStr as _,
};

use models::{
	RequestUserData,
	rbac::{ResourcePermissionType, WorkspacePermission},
	utils::ClientType,
};
use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands as _, StringCommands as _},
};
use time::OffsetDateTime;
use tokio::sync::OnceCell;

use crate::{models::redis::UserPermissionCache, prelude::*, utils::config::AppConfig};

/// A global map of Permission -> PermissionID for all permissions.
/// This is used to cache the permission IDs for faster lookup instead of
/// fetching it from the database every time.
#[doc(hidden)]
static PERMISSION_TO_ID_MAP: OnceCell<BTreeMap<Permission, Uuid>> = OnceCell::const_new();

/// Looks up the UUID for a given [`Permission`] in the database, caching
/// the full permission table on first call via [`PERMISSION_TO_ID_MAP`].
pub async fn get_permission_id(database: &mut DatabaseConnection, permission: Permission) -> Uuid {
	PERMISSION_TO_ID_MAP
		.get_or_init(async || {
			query!(
				r#"
				SELECT
					id AS "id: Uuid",
					name
				FROM
					permission;
				"#
			)
			.fetch_all(&mut *database)
			.await
			.expect("Failed to fetch permissions from the database")
			.into_iter()
			.map(|row| {
				(
					Permission::from_str(&row.name).expect("Invalid permission name"),
					row.id,
				)
			})
			.collect()
		})
		.await
		.get(&permission)
		.copied()
		.unwrap_or_else(|| {
			panic!("Permission {permission} does not exist in the database");
		})
}

/// Contains the functions to extract permissions for an API token.
mod api_token;
/// Contains the functions to extract permissions for a web dashboard JWT.
mod web_dashboard;

/// Resolve the effective permission map for an identity, keyed by its identity
/// ID.
///
/// `identity_id` is the user ID for human identities and the service account ID
/// for service accounts; `login_id` is the credential's own ID (the web login
/// ID, the API token ID, or — for service accounts, which hold a single
/// non-rotating credential — the service account ID again).
///
/// A valid cache entry short-circuits the whole thing. Otherwise this delegates
/// to the sub-module that owns the permission source for `client_type`, caches
/// what it computed, and returns it. The sub-modules are pure database reads —
/// caching lives here so every identity type gets the same invalidation
/// semantics.
#[tracing::instrument(skip(db_connection, redis_connection))]
pub async fn get_permissions_for_identity(
	db_connection: &mut DatabaseConnection,
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
	identity_id: &Uuid,
	client_type: ClientType,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	if let Some(cached) = get_cached_permissions(redis_connection, login_id, identity_id).await? {
		return Ok(cached);
	}

	let permissions = match client_type {
		// A web session and a service account both authenticate as themselves,
		// so they resolve to whatever their identity holds.
		ClientType::WebDashboard | ClientType::ServiceAccount => {
			role_derived_permissions(&mut *db_connection, identity_id).await?
		}
		ClientType::ApiToken => {
			api_token::get_permissions_for_api_token(&mut *db_connection, login_id, identity_id)
				.await?
		}
	};

	// The stored `creation_time` is what the revocation timestamps in
	// `get_cached_permissions` are compared against.
	redis_connection
		.setex(
			redis::keys::permission_for_login_id(login_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&UserPermissionCache {
				permission: permissions.clone(),
				creation_time: OffsetDateTime::now_utc(),
			})?,
		)
		.await
		.inspect_err(|err| {
			error!(
				"Error setting the permissions for the loginId `{login_id}`: `{}`",
				err
			);
		})?;

	Ok(permissions)
}

/// Gets the user data for the given token. Dispatches to the appropriate
/// module based on the token format: `patrv1.*` tokens go through
/// [`api_token`], anything else is treated as a JWT and goes through
/// [`web_dashboard`].
///
/// The resolved [`ClientType`] is available on the returned
/// [`RequestUserData`].
pub async fn get_user_data_for_token(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	if token.starts_with("patrv1.") {
		api_token::get_permissions(database, redis, config, client_ip, token).await
	} else {
		web_dashboard::get_permissions(database, redis, config, client_ip, token).await
	}
}

/// Read the cached permission map for `login_id` from Redis if it is still
/// valid. Validity is checked against four revocation timestamps (identity,
/// login, workspace, global): if any timestamp is newer than the cache
/// entry's `creation_time`, the entry is stale, gets deleted, and `None` is
/// returned so the caller recomputes from the database.
///
/// `identity_id` is the user ID for human identities and the service account
/// ID for service accounts — both share the
/// [`user_id_revocation_timestamp`][redis::keys::user_id_revocation_timestamp]
/// namespace, so rotating a service account's token invalidates its cached
/// permissions the same way revoking a user's does.
async fn get_cached_permissions(
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
	identity_id: &Uuid,
) -> Result<Option<BTreeMap<Uuid, WorkspacePermission>>, ErrorType> {
	let redis_data: Option<String> = redis_connection
		.get(redis::keys::permission_for_login_id(login_id))
		.await?;
	let Some(Ok(data)) = redis_data
		.as_deref()
		.map(serde_json::from_str::<UserPermissionCache>)
	else {
		return Ok(None);
	};

	// Check whether the data stored in redis is still valid
	// Simple example: When a user has their permissions stored in Redis, and they
	// have been removed from a workspace, that data in redis should be considered
	// invalid. This check is to ensure that the data stored in redis is still
	// valid.
	// So when a user's permissions are updated (like being removed from a
	// workspace), a timestamp is set in redis. When a request is processed, if this
	// timestamp exists in Redis, and the data inserted into redis was inserted
	// after this timestamp, it is considered valid.

	// Check user revocation, then loginId revocation, then workspace ID revocation
	let is_valid = 'validity: {
		let revoked = redis_connection
			.get::<Option<String>>(redis::keys::user_id_revocation_timestamp(identity_id))
			.await?
			.and_then(|s| s.parse::<i128>().ok())
			.and_then(|time| OffsetDateTime::from_unix_timestamp_nanos(time).ok())
			.filter(|time| {
				// If the timestamp exists, and the token was inserted into Redis before the
				// timestamp, then the data in Redis is considered invalid
				data.creation_time < *time
			})
			.is_some();

		if revoked {
			break 'validity false;
		}

		let revoked = redis_connection
			.get::<Option<String>>(redis::keys::login_id_revocation_timestamp(login_id))
			.await?
			.and_then(|s| s.parse::<i128>().ok())
			.and_then(|time| OffsetDateTime::from_unix_timestamp_nanos(time).ok())
			.filter(|time| {
				// If the timestamp exists, and the token was inserted into Redis before the
				// timestamp, then the data in Redis is considered invalid
				data.creation_time < *time
			})
			.is_some();

		if revoked {
			break 'validity false;
		}

		for workspace_id in data.permission.keys() {
			let revoked = redis_connection
				.get::<Option<String>>(redis::keys::workspace_id_revocation_timestamp(workspace_id))
				.await?
				.and_then(|s| s.parse::<i128>().ok())
				.and_then(|time| OffsetDateTime::from_unix_timestamp_nanos(time).ok())
				.filter(|time| {
					// If the timestamp exists, and the token was inserted into Redis before the
					// timestamp, then the data in Redis is considered invalid
					data.creation_time < *time
				})
				.is_some();

			if revoked {
				break 'validity false;
			}
		}

		let revoked = redis_connection
			.get::<Option<String>>(redis::keys::global_revocation_timestamp())
			.await?
			.and_then(|s| s.parse::<i128>().ok())
			.and_then(|time| OffsetDateTime::from_unix_timestamp_nanos(time).ok())
			.filter(|time| {
				// If the timestamp exists, and the token was inserted into Redis before the
				// timestamp, then the data in Redis is considered invalid
				data.creation_time < *time
			})
			.is_some();

		if revoked {
			break 'validity false;
		}

		// None of the revocation timestamps exist, so the data in Redis is
		// valid and can be used
		true
	};

	if is_valid {
		Ok(Some(data.permission))
	} else {
		// The data in redis is not valid anymore. It probably is expired due to a
		// permission change, so delete it from Redis, and proceed to fetch the
		// permissions from the database again. This ensures that the permissions are up
		// to date, even if the cache is stale.
		_ = redis_connection
			.del(redis::keys::permission_for_login_id(login_id))
			.await;
		Ok(None)
	}
}

/// Compute the permission map an identity holds in its own right: workspaces it
/// owns outright, plus everything granted by the roles it is assigned through
/// `workspace_member`.
///
/// Shared by web logins and service accounts, which differ only in that a
/// service account never owns a workspace (`workspace.super_admin_id`
/// references a user), so the ownership query simply returns nothing for one.
/// API tokens deliberately do *not* use this directly — a token carries the
/// scope captured when it was minted — but they intersect against it, so this
/// is also the upper bound in [`api_token::get_permissions_for_api_token`].
///
/// Caching is the caller's job — see [`get_permissions_for_identity`].
#[tracing::instrument(skip(db_connection))]
pub async fn role_derived_permissions(
	db_connection: &mut DatabaseConnection,
	identity_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	let mut identity_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			id AS "workspace_id!"
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		identity_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		identity_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	query!(
		r#"
		SELECT
			workspace_member.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_exclude.resource_id AS "resource_id?"
		FROM
			workspace_member
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = workspace_member.role_id AND
			role_resource_permissions_type.permission_type = 'exclude'
		LEFT JOIN
			role_resource_permissions_exclude
		ON
			role_resource_permissions_exclude.role_id = workspace_member.role_id
		WHERE
			workspace_member.identity_id = $1;
		"#,
		identity_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = identity_permissions
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
			workspace_member.workspace_id AS "workspace_id!",
			role_resource_permissions_type.permission_id AS "permission_id!",
			role_resource_permissions_include.resource_id AS "resource_id?"
		FROM
			workspace_member
		INNER JOIN
			role_resource_permissions_type
		ON
			role_resource_permissions_type.role_id = workspace_member.role_id AND
			role_resource_permissions_type.permission_type = 'include'
		LEFT JOIN
			role_resource_permissions_include
		ON
			role_resource_permissions_include.role_id = workspace_member.role_id
		WHERE
			workspace_member.identity_id = $1;
		"#,
		identity_id as _,
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = identity_permissions
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

	Ok(identity_permissions)
}
