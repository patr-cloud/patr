/// Registry authentication layer.
///
/// This layer handles authentication for registry endpoints that require it.
/// It extracts the Authorization header (Bearer token), validates it as an API
/// token, and converts a `RegistryRequestWithConnections` to an
/// `AuthenticatedRegistryRequest`.
///
/// On authentication failure, it returns a 401 Unauthorized response with the
/// WWW-Authenticate header as required by the OCI Distribution Specification.
use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use models::{RequestUserData, utils::HasHeader};
use oci_spec::distribution::ErrorCode;
use preprocess::Preprocessable;
use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands, StringCommands},
};
use time::OffsetDateTime;
use tower::{Layer, Service};

use crate::{models::redis::UserPermissionCache, routes::registry_patr_cloud::prelude::*};

/// Layer that authenticates registry requests using API tokens.
///
/// This layer:
/// 1. Extracts the Authorization header (Bearer token)
/// 2. Validates the token as an API token (format:
///    patrv1.{refresh_token}.{login_id})
/// 3. Verifies the token against the database
/// 4. Checks token expiration, revocation, and IP restrictions
/// 5. Loads user permissions from Redis cache or database
/// 6. Converts `RegistryRequestWithConnections` to
///    `AuthenticatedRegistryRequest`
/// 7. Returns 401 with WWW-Authenticate header on failure
#[derive(Clone)]
pub struct RegistryAuthenticationLayer<E>
where
	E: RegistryEndpoint,
{
	phantom: PhantomData<E>,
}

impl<E> RegistryAuthenticationLayer<E>
where
	E: RegistryEndpoint,
{
	/// Create a new registry authentication layer.
	pub fn new() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RegistryAuthenticationLayer<E>
where
	E: RegistryEndpoint,
{
	type Service = RegistryAuthenticationService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryAuthenticationService {
			inner,
			phantom: PhantomData,
		}
	}
}

/// Tower service that authenticates registry requests.
///
/// This service is created by `RegistryAuthenticationLayer` and handles the
/// authentication logic for API tokens.
#[derive(Clone)]
pub struct RegistryAuthenticationService<S, E>
where
	E: RegistryEndpoint,
{
	inner: S,
	phantom: PhantomData<E>,
}

impl<'a, S, E> Service<RegistryAppRequest<'a, E>> for RegistryAuthenticationService<S, E>
where
	for<'b> S: Service<
			AuthenticatedRegistryAppRequest<'b, E>,
			Response = RegistryResponse<E>,
			Error = RegistryError,
		> + Clone
		+ 'a,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
	E::RequestHeaders: HasHeader<BearerToken>,
{
	type Error = RegistryError;
	type Response = RegistryResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>> + 'a;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "RegistryAuthenticationService")]
	fn call(&mut self, req: RegistryAppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();

		async move {
			debug!("Authenticating registry request");
			let BearerToken(token) = req.request.headers.get_header();
			let token = token.token();

			trace!("Parsing authentication header as an API token");

			// Parse API token format: patrv1.{refresh_token}.{login_id}
			let (refresh_token, login_id) = token
				.strip_prefix("patrv1.")
				.ok_or_else(|| {
					warn!("Invalid API token format: missing patrv1 prefix");
					RegistryError::unauthorized("Invalid API token format")
				})?
				.split_once('.')
				.ok_or_else(|| {
					warn!("Invalid API token format: missing delimiter");
					RegistryError::unauthorized("Invalid API token format")
				})?;

			let refresh_token = Uuid::parse_str(refresh_token).map_err(|err| {
				warn!("Cannot parse refresh token as UUID: {}", err);
				RegistryError::unauthorized("Invalid API token format")
			})?;
			trace!("Refresh token parsed as UUID");

			let login_id = Uuid::parse_str(login_id).map_err(|err| {
				warn!("Cannot parse login_id as UUID: {}", err);
				RegistryError::unauthorized("Invalid API token format")
			})?;
			trace!("Login ID parsed as UUID");

			info!("Extracting information about API token");

			// Query database for token information
			let Some(token_record) = query!(
				r#"
				SELECT
					user_api_token.token_id,
					user_api_token.user_id,
					user_api_token.token_hash,
					user_api_token.token_nbf,
					user_api_token.token_exp,
					user_api_token.allowed_ips,
					user_api_token.revoked,
					"user".*
				FROM
					user_api_token
				INNER JOIN
					user_login
				ON
					user_api_token.token_id = user_login.login_id
				INNER JOIN
					"user"
				ON
					user_api_token.user_id = "user".id
				WHERE
					user_api_token.token_id = $1 AND
					user_login.login_type = 'api_token';
				"#,
				login_id as _
			)
			.fetch_optional(&mut **req.database)
			.await
			.map_err(|err| {
				error!("Database error while fetching API token: {}", err);
				RegistryError::new(ErrorCode::Unsupported, "Internal server error")
			})?
			else {
				warn!("API token not found in database");
				return Err(RegistryError::unauthorized("Invalid API token"));
			};
			trace!("Token extracted from database");

			// Check token NBF (not before)
			if let Some(nbf) = token_record.token_nbf {
				trace!("Token has an NBF");
				if OffsetDateTime::now_utc() < nbf {
					info!("API token is not valid yet");
					return Err(RegistryError::unauthorized("API token not valid yet"));
				}
			} else {
				trace!("Token does not have an NBF");
			}
			trace!("Token passed NBF check");

			// Check token EXP (expiration)
			if let Some(exp) = token_record.token_exp {
				trace!("Token has an EXP");
				if OffsetDateTime::now_utc() > exp {
					info!("API token has expired");
					return Err(RegistryError::unauthorized("API token has expired"));
				}
			} else {
				trace!("Token does not have an EXP");
			}
			trace!("Token passed EXP check");

			// Check token revocation
			if let Some(revoked) = token_record.revoked {
				trace!("Token has a revoked timestamp");
				if OffsetDateTime::now_utc() > revoked {
					info!("API token has been revoked");
					return Err(RegistryError::unauthorized("API token has been revoked"));
				}
			} else {
				trace!("Token does not have a revoked timestamp");
			}
			trace!("Token passed revoked timestamp check");

			// Check IP restrictions
			if let Some(allowed_ips) = token_record.allowed_ips &&
				!allowed_ips
					.iter()
					.any(|ip_network| ip_network.contains(req.client_ip))
			{
				info!("API token not accessed from an allowed IP Address");
				return Err(RegistryError::unauthorized(
					"API token not allowed from this IP address",
				));
			}

			// Verify token hash using Argon2
			let Ok(password_hash) = PasswordHash::new(&token_record.token_hash) else {
				error!("Unable to parse password hash: {}", token_record.token_hash);
				return Err(RegistryError::new(
					ErrorCode::Unsupported,
					"Internal server error: password hash parsing failed",
				));
			};

			let success = Argon2::new_with_secret(
				req.config.password_pepper.as_bytes(),
				Algorithm::Argon2id,
				Version::V0x13,
				constants::HASHING_PARAMS,
			)
			.map_err(|err| {
				error!("Failed to create Argon2 instance: {}", err);
				RegistryError::new(ErrorCode::Unsupported, "Internal server error")
			})?
			.verify_password(refresh_token.as_bytes(), &password_hash)
			.is_ok();

			if !success {
				warn!("API token has invalid refresh token");
				return Err(RegistryError::unauthorized("Invalid API token"));
			}
			info!("API token valid");

			// Get user permissions from Redis cache or database
			let permissions = get_permissions_for_login_id(
				req.database,
				req.redis,
				&login_id,
				&token_record.user_id.into(),
			)
			.await
			.map_err(|err| {
				error!("Failed to get permissions: {}", err);
				RegistryError::new(ErrorCode::Unsupported, "Internal server error")
			})?;

			// Build user data
			let user_data = RequestUserData::builder()
				.id(token_record.user_id)
				.username(token_record.username)
				.first_name(token_record.first_name)
				.last_name(token_record.last_name)
				.created(token_record.created)
				.login_id(token_record.token_id)
				.permissions(permissions)
				.build();

			debug!("User authenticated successfully: {}", user_data.id);

			// Create authenticated request
			let request = AuthenticatedRegistryAppRequest {
				request: req.request,
				database: req.database,
				redis: req.redis,
				s3: req.s3,
				client_ip: req.client_ip,
				user_data,
				config: req.config,
			};

			// Call inner service with authenticated request
			inner.call(request).await
		}
	}
}

/// Get all the permissions for a given login ID.
///
/// This function first checks the Redis cache, and if the data is not found,
/// it queries the database and then stores the result in the Redis cache.
///
/// This is a simplified version of the function from
/// `api/src/utils/layers/authenticator.rs` adapted for registry use.
#[tracing::instrument(skip(db_connection, redis_connection))]
async fn get_permissions_for_login_id(
	db_connection: &mut DatabaseConnection,
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<std::collections::BTreeMap<Uuid, models::rbac::WorkspacePermission>, ErrorType> {
	use std::collections::{BTreeMap, BTreeSet};

	use models::rbac::{ResourcePermissionType, WorkspacePermission};

	// Try to get permissions from Redis cache
	let redis_data: Option<String> = redis_connection
		.get(redis::keys::permission_for_login_id(login_id))
		.await?;

	if let Some(Ok(data)) = redis_data
		.as_deref()
		.map(serde_json::from_str::<UserPermissionCache>)
	{
		// Check whether the data stored in redis is still valid
		'is_valid: {
			let revoked = redis_connection
				.get::<_, Option<i64>>(redis::keys::user_id_revocation_timestamp(user_id))
				.await?
				.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
				.filter(|time| data.creation_time < *time)
				.is_some();

			if revoked {
				break 'is_valid;
			}

			let revoked = redis_connection
				.get::<_, Option<i64>>(redis::keys::login_id_revocation_timestamp(login_id))
				.await?
				.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
				.filter(|time| data.creation_time < *time)
				.is_some();

			if revoked {
				_ = redis_connection
					.del(redis::keys::login_id_revocation_timestamp(login_id))
					.await;
				break 'is_valid;
			}

			for workspace_id in data.permission.keys() {
				let revoked = redis_connection
					.get::<_, Option<i64>>(redis::keys::workspace_id_revocation_timestamp(
						workspace_id,
					))
					.await?
					.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
					.filter(|time| data.creation_time < *time)
					.is_some();

				if revoked {
					_ = redis_connection
						.del(redis::keys::workspace_id_revocation_timestamp(workspace_id))
						.await;
					break 'is_valid;
				}
			}

			let revoked = redis_connection
				.get::<_, Option<i64>>(redis::keys::global_revocation_timestamp())
				.await?
				.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
				.filter(|time| data.creation_time < *time)
				.is_some();

			if revoked {
				_ = redis_connection
					.del(redis::keys::global_revocation_timestamp())
					.await;
				break 'is_valid;
			}

			// None of the revocation timestamps exist, so the data in Redis is valid
			return Ok(data.permission);
		};
	}

	// Cache miss or invalid - query database
	let mut workspace_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

	// Get super admin workspaces
	query!(
		r#"
		SELECT DISTINCT
			COALESCE(
				user_api_token_workspace_super_admin.workspace_id,
				workspace.id
			) AS "workspace_id"
		FROM
			user_login
		LEFT JOIN
			user_api_token_workspace_super_admin
		ON
			user_login.login_type = 'api_token' AND
			user_api_token_workspace_super_admin.token_id = user_login.login_id
		LEFT JOIN
			workspace
		ON
			user_login.login_type = 'web_login' AND
			workspace.super_admin_id = user_login.user_id
		WHERE
			user_login.login_id = $1;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.filter_map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		workspace_permissions.insert(workspace_id.into(), WorkspacePermission::SuperAdmin);
	});

	// Get excluded permissions
	query!(
		r#"
		SELECT
			COALESCE(
				user_api_token_resource_permissions_exclude.workspace_id,
				workspace_user.workspace_id
			) AS "workspace_id",
			COALESCE(
				user_api_token_resource_permissions_exclude.resource_id,
				role_resource_permissions_exclude.resource_id
			) AS "resource_id",
			COALESCE(
				user_api_token_resource_permissions_exclude.permission_id,
				role_resource_permissions_exclude.permission_id
			) AS "permission_id"
		FROM
			user_login
		LEFT JOIN
			user_api_token_resource_permissions_exclude
		ON
			user_login.login_type = 'api_token' AND
			user_api_token_resource_permissions_exclude.token_id = user_login.login_id
		LEFT JOIN
			workspace_user
		ON
			workspace_user.user_id = user_login.user_id
		LEFT JOIN
			role_resource_permissions_exclude
		ON
			role_resource_permissions_exclude.role_id = workspace_user.role_id
		WHERE
			user_login.login_id = $1;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.filter_map(|row| row.workspace_id.zip(row.resource_id).zip(row.permission_id))
	.for_each(|((workspace_id, resource_id), permission_id)| {
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
							"Found include permissions before include is even called. This should be impossible!"
						);
					}
					ResourcePermissionType::Exclude(resources) => {
						resources.insert(resource_id.into());
					}
				}
			}
		}
	});

	// Get included permissions
	query!(
		r#"
		SELECT
			COALESCE(
				user_api_token_resource_permissions_include.workspace_id,
				workspace_user.workspace_id
			) AS "workspace_id",
			COALESCE(
				user_api_token_resource_permissions_include.resource_id,
				role_resource_permissions_include.resource_id
			) AS "resource_id",
			COALESCE(
				user_api_token_resource_permissions_include.permission_id,
				role_resource_permissions_include.permission_id
			) AS "permission_id"
		FROM
			user_login
		LEFT JOIN
			user_api_token_resource_permissions_include
		ON
			user_login.login_type = 'api_token' AND
			user_api_token_resource_permissions_include.token_id = user_login.login_id
		LEFT JOIN
			workspace_user
		ON
			workspace_user.user_id = user_login.user_id
		LEFT JOIN
			role_resource_permissions_include
		ON
			role_resource_permissions_include.role_id = workspace_user.role_id
		WHERE
			user_login.login_id = $1;
		"#,
		login_id as _
	)
	.fetch_all(&mut *db_connection)
	.await?
	.into_iter()
	.filter_map(|row| row.workspace_id.zip(row.resource_id).zip(row.permission_id))
	.for_each(|((workspace_id, resource_id), permission_id)| {
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

	// Store in Redis cache
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
