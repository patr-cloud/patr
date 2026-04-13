use std::{collections::BTreeMap, net::IpAddr, str::FromStr as _};

use models::{RequestUserData, rbac::WorkspacePermission};
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
/// Contains the functions to extract permissions for a service account token.
mod service_account;
/// Contains the functions to extract permissions for a web dashboard JWT.
mod web_dashboard;

/// The kind of credential a request authenticated with. Each variant has its
/// own permission source, so the identity ID alone is not enough to know where
/// to read permissions from — [`get_permissions_for_identity`] needs both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityTokenType {
	/// A web dashboard session (JWT). Permissions come from the user's
	/// workspace ownership and role memberships.
	WebLogin,
	/// A user API token. Permissions are the user's current permissions
	/// intersected with the scope declared when the token was minted.
	ApiToken,
	/// A service account token. Permissions come from the roles assigned
	/// directly to the service account.
	ServiceAccount,
}

/// Resolve the effective permission map for an identity, keyed by its identity
/// ID.
///
/// `identity_id` is the user ID for human identities and the service account ID
/// for service accounts; `login_id` is the credential's own ID (the web login
/// ID, the API token ID, or — for service accounts, which hold a single
/// non-rotating credential — the service account ID again).
///
/// A valid cache entry short-circuits the whole thing. Otherwise this delegates
/// to the sub-module that owns the permission source for `token_type`, caches
/// what it computed, and returns it. The sub-modules are pure database reads —
/// caching lives here so every identity type gets the same invalidation
/// semantics.
#[tracing::instrument(skip(db_connection, redis_connection))]
pub async fn get_permissions_for_identity(
	db_connection: &mut DatabaseConnection,
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
	identity_id: &Uuid,
	token_type: IdentityTokenType,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	if let Some(cached) = get_cached_permissions(redis_connection, login_id, identity_id).await? {
		return Ok(cached);
	}

	let permissions = match token_type {
		IdentityTokenType::WebLogin => {
			web_dashboard::get_permissions_for_web_login(&mut *db_connection, identity_id).await?
		}
		IdentityTokenType::ApiToken => {
			api_token::get_permissions_for_api_token(&mut *db_connection, login_id, identity_id)
				.await?
		}
		IdentityTokenType::ServiceAccount => {
			service_account::get_permissions_for_service_account(&mut *db_connection, identity_id)
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
