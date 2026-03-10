use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	str::FromStr as _,
};

use models::{
	RequestUserData,
	rbac::{ResourcePermissionType, WorkspacePermission},
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

/// Gets the user data for the given token based on the allowed client type.
/// This function delegates the permission extraction to the appropriate module
/// based on whether the token is for an API token or a web dashboard session.
///
/// For a given token, it gets all the information of the user that the login ID
/// has (based on that token).
pub async fn get_user_data_for_token(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	allowed_client_type: ClientType,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	match allowed_client_type {
		ClientType::ApiToken => {
			api_token::get_permissions(database, redis, config, client_ip, token).await
		}
		ClientType::WebDashboard => {
			web_dashboard::get_permissions(database, redis, config, client_ip, token).await
		}
	}
}

/// Get all the permissions for a given login ID. This will first check the
/// Redis cache, and if the data is not found, it will query the database and
/// then store the result in the Redis cache.
#[tracing::instrument(skip(db_connection, redis_connection))]
async fn get_permissions_for_login_id(
	db_connection: &mut DatabaseConnection,
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	let redis_data: Option<String> = redis_connection
		.get(redis::keys::permission_for_login_id(login_id))
		.await?;
	if let Some(Ok(data)) = redis_data
		.as_deref()
		.map(serde_json::from_str::<UserPermissionCache>)
	{
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
				.get::<Option<String>>(redis::keys::user_id_revocation_timestamp(user_id))
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
					.get::<Option<String>>(redis::keys::workspace_id_revocation_timestamp(
						workspace_id,
					))
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
			return Ok(data.permission);
		} else {
			// The data in redis is not valid anymore. It probably is expired due to a
			// permission change, so delete it from Redis, and proceed to fetch the
			// permissions from the database again. This ensures that the permissions are up
			// to date, even if the cache is stale.
			_ = redis_connection
				.del(redis::keys::permission_for_login_id(login_id))
				.await;
		}
	}

	let mut workspace_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!"
		FROM (
			/* API token super-admin permissions */
			SELECT
				user_api_token_workspace_super_admin.workspace_id
			FROM
				user_login
			INNER JOIN
				user_api_token_workspace_super_admin
			ON
				user_api_token_workspace_super_admin.token_id = user_login.login_id
			WHERE
				user_login.login_id = $1 AND
				user_login.login_type = 'api_token'

			UNION ALL

			/* Web login super-admin permissions (workspace owners) */
			SELECT
				workspace.id AS workspace_id
			FROM
				user_login
			INNER JOIN
				workspace
			ON
				workspace.super_admin_id = user_login.user_id
			WHERE
				user_login.login_id = $1 AND
				user_login.login_type = 'web_login'
		) AS super_admins;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		workspace_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	// Once all super-admins are added, add the excludes, then remove the includes
	query!(
		r#"
		SELECT
			workspace_id AS "workspace_id!",
			permission_id AS "permission_id!",
			resource_id
		FROM (
			/* API token exclude permissions */
			SELECT
				user_api_token_resource_permissions_type.workspace_id,
				user_api_token_resource_permissions_type.permission_id,
				user_api_token_resource_permissions_exclude.resource_id
			FROM
				user_login
			INNER JOIN
				user_api_token_resource_permissions_type
			ON
				user_api_token_resource_permissions_type.token_id = user_login.login_id AND
				user_api_token_resource_permissions_type.resource_permission_type = 'exclude'
			LEFT JOIN
				user_api_token_resource_permissions_exclude
			ON
				user_api_token_resource_permissions_exclude.token_id = user_login.login_id
			WHERE
				user_login.login_id = $1 AND
				user_login.login_type = 'api_token'

			UNION ALL

			/* Role-based exclude permissions */
			SELECT
				workspace_user.workspace_id,
				role_resource_permissions_type.permission_id,
				role_resource_permissions_exclude.resource_id
			FROM
				user_login
			INNER JOIN
				workspace_user
			ON
				workspace_user.user_id = user_login.user_id
			INNER JOIN
				role_resource_permissions_type
			ON
				role_resource_permissions_type.role_id = workspace_user.role_id AND
				role_resource_permissions_type.permission_type = 'exclude'
			LEFT JOIN
				role_resource_permissions_exclude
			ON
				role_resource_permissions_exclude.role_id = workspace_user.role_id
			WHERE
				user_login.login_id = $1
		) AS excludes;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = workspace_permissions
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
			workspace_id AS "workspace_id!",
			permission_id AS "permission_id!",
			resource_id
		FROM (
			/* API token include permissions */
			SELECT
				user_api_token_resource_permissions_type.workspace_id,
				user_api_token_resource_permissions_type.permission_id,
				user_api_token_resource_permissions_include.resource_id
			FROM
				user_login
			INNER JOIN
				user_api_token_resource_permissions_type
			ON
				user_api_token_resource_permissions_type.token_id = user_login.login_id AND
				user_api_token_resource_permissions_type.resource_permission_type = 'include'
			LEFT JOIN
				user_api_token_resource_permissions_include
			ON
				user_api_token_resource_permissions_include.token_id = user_login.login_id
			WHERE
				user_login.login_id = $1 AND
				user_login.login_type = 'api_token'

			UNION ALL

			/* Role-based include permissions */
			SELECT
				workspace_user.workspace_id,
				role_resource_permissions_type.permission_id,
				role_resource_permissions_include.resource_id
			FROM
				user_login
			INNER JOIN
				workspace_user
			ON
				workspace_user.user_id = user_login.user_id
			INNER JOIN
				role_resource_permissions_type
			ON
				role_resource_permissions_type.role_id = workspace_user.role_id AND
				role_resource_permissions_type.permission_type = 'include'
			LEFT JOIN
				role_resource_permissions_include
			ON
				role_resource_permissions_include.role_id = workspace_user.role_id
			WHERE
				user_login.login_id = $1
		) AS includes;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.for_each(|(workspace_id, permission_id, resource_id)| {
		let permissions = workspace_permissions
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

	redis_connection
		.setex(
			redis::keys::permission_for_login_id(login_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&UserPermissionCache {
				permission: workspace_permissions.clone(),
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

	Ok(workspace_permissions)
}
