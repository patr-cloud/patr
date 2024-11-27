use std::{
	collections::{BTreeMap, BTreeSet},
	future::Future,
	marker::PhantomData,
	ops::Sub,
	task::{Context, Poll},
};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::{
	rbac::{ResourcePermissionType, WorkspacePermission},
	utils::{AppAuthentication, BearerToken, HasHeader},
	RequestUserData,
};
use preprocess::Preprocessable;
use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands, StringCommands},
};
use time::OffsetDateTime;
use tower::{Layer, Service};

use crate::{
	models::{access_token_data::AccessTokenData, redis::UserPermissionCache},
	prelude::*,
};

/// The type of client used for a request. This is used to determine
/// which authentication method to use, based on if the API call is made by our
/// web dashboard or by a third party application using the API token. This is
/// required because some endpoints are only accessible by the web dashboard,
/// and some are only accessible by third party applications. For example, you
/// cannot change your password, or create a new user using the API token, but
/// you can do so using the web dashboard.
#[derive(Debug, Clone, Copy)]
pub enum ClientType {
	/// The request is authenticated using a JWT from the web dashboard
	WebDashboard,
	/// The request is authenticated using an API token
	ApiToken,
}

/// The [`tower::Layer`] used to authenticate requests. This will parse the
/// [`BearerToken`] header and verify it against the database. If the token is
/// valid, the [`RequestUserData`] will be added to the request. All subsequent
/// underlying layers will recieve an [`AuthenticatedAppRequest`] with the
/// appropriate [`RequestUserData`] filled.
pub struct AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The type of client that is allowed to make the request
	client_type: ClientType,
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<E> AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize an authentication layer
	pub fn new(client_type: ClientType) -> Self {
		Self {
			endpoint: PhantomData,
			client_type,
		}
	}
}

impl<E, S> Layer<S> for AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AuthenticatedAppRequest<'a, E>>,
{
	type Service = AuthenticationService<E::Authenticator, E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthenticationService {
			inner,
			client_type: self.client_type,
			authenticator: PhantomData,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuthenticationLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
			client_type: self.client_type,
		}
	}
}

/// The underlying service that runs when the [`AuthenticationLayer`] is used.
pub struct AuthenticationService<A, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request is authenticated
	inner: S,
	/// The type of client that is allowed to make the request
	client_type: ClientType,
	/// The type of authenticator that will be used to authenticate the request
	authenticator: PhantomData<A>,
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AppRequest<'a, E>> for AuthenticationService<AppAuthentication<E>, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestHeaders: HasHeader<BearerToken>,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	#[instrument(skip(self, req), name = "AuthenticatorService")]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		let client_type = self.client_type;
		async move {
			trace!("Authenticating request");
			let BearerToken(token) = req.request.headers.get_header();
			let token = token.token();

			let user_data = match client_type {
				ClientType::ApiToken => {
					trace!("Parsing authentication header as an API token");
					let (refresh_token, login_id) = token
						.strip_prefix("patrv1.")
						.ok_or_else(|| {
							warn!("Invalid API token provided: {}", token);
							ErrorType::MalformedApiToken
						})?
						.split_once('.')
						.ok_or_else(|| {
							warn!("Invalid API token provided: {}", token);
							ErrorType::MalformedApiToken
						})?;

					let refresh_token = Uuid::parse_str(refresh_token).map_err(|err| {
						warn!("Invalid API token provided: {}", token);
						warn!(
							"Cannot parse refresh token `{}` as UUID: {}",
							refresh_token, err
						);
						ErrorType::MalformedApiToken
					})?;
					trace!("Refresh token parsed as UUID");

					let login_id = Uuid::parse_str(login_id).map_err(|err| {
						warn!("Invalid API token provided: {}", token);
						warn!("Cannot parse loginId `{}` as UUID: {}", login_id, err);
						ErrorType::MalformedApiToken
					})?;
					trace!("Login ID parsed as UUID");

					info!("Extracting information about API token");
					let Some(token) = query!(
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
					.fetch_optional(&mut **req.database) // What the actual fuck?
					.await?
					else {
						warn!("API token not found");
						// No specific error for API token not found, since we don't want to leak
						// information about whether a loginId is valid or if it's expired
						return Err(ErrorType::AuthorizationTokenInvalid);
					};
					trace!("Token extracted from database");

					if let Some(nbf) = token.token_nbf {
						trace!("Token has an NBF");
						if OffsetDateTime::now_utc() < nbf {
							info!("API token is not valid yet");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					} else {
						trace!("Token does not have an NBF");
					}
					trace!("Token passed NBF check");

					if let Some(exp) = token.token_exp {
						trace!("Token has an EXP");
						if OffsetDateTime::now_utc() > exp {
							info!("API token has expired");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					} else {
						trace!("Token does not have an EXP");
					}
					trace!("Token passed EXP check");

					if let Some(revoked) = token.revoked {
						trace!("Token has a revoked timestamp");
						if OffsetDateTime::now_utc() > revoked {
							info!("API token has been revoked");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					} else {
						trace!("Token does not have a revoked timestamp");
					}
					trace!("Token passed revoked timestamp check");

					if let Some(allowed_ips) = token.allowed_ips {
						if !allowed_ips
							.iter()
							.any(|ip_network| ip_network.contains(req.client_ip))
						{
							info!("API token not accessed from an allowed IP Address");
							return Err(ErrorType::DisallowedIpAddressForApiToken);
						}
					}

					let Ok(password_hash) = PasswordHash::new(&token.token_hash) else {
						error!("Unable to parse password hash: {}", token.token_hash);
						return Err(ErrorType::server_error("password hash parsing failed"));
					};
					let success = Argon2::new_with_secret(
						req.config.password_pepper.as_bytes(),
						Algorithm::Argon2id,
						Version::V0x13,
						constants::HASHING_PARAMS,
					)
					.map_err(ErrorType::server_error)?
					.verify_password(refresh_token.as_bytes(), &password_hash)
					.is_ok();

					if !success {
						warn!("API token has invalid refresh token");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}
					info!("API token valid");

					let permissions = get_permissions_for_login_id(
						req.database,
						req.redis,
						&login_id,
						&token.user_id.into(),
					)
					.await?;

					RequestUserData::builder()
						.id(token.user_id)
						.username(token.username)
						.first_name(token.first_name)
						.last_name(token.last_name)
						.created(token.created)
						.login_id(token.token_id)
						.permissions(permissions)
						.build()
				}
				ClientType::WebDashboard => {
					trace!("Parsing authentication header as a JWT");

					let TokenData {
						header: _,
						claims:
							AccessTokenData {
								iss,
								sub,
								aud,
								exp,
								nbf,
								iat: _,
								jti,
							},
					} = jsonwebtoken::decode(
						token,
						&DecodingKey::from_secret(req.config.jwt_secret.as_ref()),
						&{
							let mut validation = Validation::default();

							// We'll manually do this
							validation.validate_exp = false;
							validation.validate_nbf = false;
							validation.validate_aud = false;

							validation
						},
					)
					.map_err(|err| {
						warn!("Invalid JWT provided: {}", err);
						ErrorType::MalformedAccessToken
					})?;
					trace!("Authentication header is a valid JWT");

					if iss != constants::JWT_ISSUER {
						warn!("Invalid JWT issuer: {}", iss);
						return Err(ErrorType::MalformedAccessToken);
					}
					trace!("JWT issuer valid");

					// The token should have been issued within the last `REFRESH_TOKEN_VALIDITY`
					// duration
					if OffsetDateTime::now_utc()
						.sub(jti.get_timestamp().ok_or(ErrorType::MalformedAccessToken)?) >
						AccessTokenData::REFRESH_TOKEN_VALIDITY
					{
						warn!("JWT is too old");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}
					trace!("JWT JTI valid");

					if OffsetDateTime::now_utc() < nbf {
						warn!("JWT is not valid yet");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}
					trace!("JWT NBF valid");

					if OffsetDateTime::now_utc() > exp {
						warn!("JWT has expired");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}
					trace!("JWT EXP valid");

					let Some(user) = query! {
						r#"
						SELECT
							"user".*,
							web_login.token_expiry
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
						sub as _
					}
					.fetch_optional(&mut **req.database)
					.await?
					else {
						warn!("web login not found");
						// No specific error for API token not found, since we don't want to leak
						// information about whether a loginId is valid or if it's expired
						return Err(ErrorType::AuthorizationTokenInvalid);
					};
					trace!("Web login exists in the database");

					if OffsetDateTime::now_utc() > user.token_expiry {
						warn!("Web login has expired");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}

					if !aud
						.clone()
						.into_iter()
						.any(|item| item == constants::PATR_JWT_AUDIENCE)
					{
						warn!(
							"Invalid JWT audience: `{}`",
							match aud {
								OneOrMore::One(aud) => aud,
								OneOrMore::Multiple(aud) => format!("[{}]", aud.join(", ")),
							}
						);
						return Err(ErrorType::MalformedAccessToken);
					}

					let permissions = get_permissions_for_login_id(
						req.database,
						req.redis,
						&sub,
						&user.id.into(),
					)
					.await?;

					RequestUserData::builder()
						.id(user.id)
						.username(user.username)
						.first_name(user.first_name)
						.last_name(user.last_name)
						.created(user.created)
						.login_id(sub)
						.permissions(permissions)
						.build()
				}
			};

			let AppRequest {
				request,
				database,
				redis,
				client_ip,
				config,
			} = req;
			let req = AuthenticatedAppRequest {
				request,
				database,
				redis,
				client_ip,
				config,
				user_data,
			};
			inner.call(req).await
		}
	}
}

impl<A, E, S> Clone for AuthenticationService<A, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client_type: self.client_type,
			authenticator: PhantomData,
			endpoint: PhantomData,
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
		'is_valid: {
			let revoked = redis_connection
				.get::<_, Option<i64>>(redis::keys::user_id_revocation_timestamp(user_id))
				.await?
				.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
				.filter(|time| {
					// If the timestamp exists, and the token was inserted into Redis before the
					// timestamp, then the data in Redis is considered invalid
					data.creation_time < *time
				})
				.is_some();

			if revoked {
				break 'is_valid;
			}

			let revoked = redis_connection
				.get::<_, Option<i64>>(redis::keys::login_id_revocation_timestamp(login_id))
				.await?
				.and_then(|time| OffsetDateTime::from_unix_timestamp(time).ok())
				.filter(|time| {
					// If the timestamp exists, and the token was inserted into Redis before the
					// timestamp, then the data in Redis is considered invalid
					data.creation_time < *time
				})
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
					.filter(|time| {
						// If the timestamp exists, and the token was inserted into Redis before the
						// timestamp, then the data in Redis is considered invalid
						data.creation_time < *time
					})
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
				.filter(|time| {
					// If the timestamp exists, and the token was inserted into Redis before the
					// timestamp, then the data in Redis is considered invalid
					data.creation_time < *time
				})
				.is_some();

			if revoked {
				_ = redis_connection
					.del(redis::keys::global_revocation_timestamp())
					.await;
				break 'is_valid;
			}

			// None of the revocation timestamps exist, so the data in Redis is
			// valid and can be used
		};

		return Ok(data.permission);
	}

	let mut workspace_permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

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

	// Once all super-admins are added, add the excludes, then remove the includes
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
							"Found include permissions before include is even called. This should be possible!"
						);
					}
					ResourcePermissionType::Exclude(resources) => {
						resources.insert(resource_id.into());
					}
				}
			}
		}
	});

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
