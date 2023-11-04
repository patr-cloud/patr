use std::{
	collections::BTreeMap,
	future::Future,
	marker::PhantomData,
	ops::Sub,
	task::{Context, Poll},
};

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier, Version};
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::{
	permission::WorkspacePermission,
	utils::{AppAuthentication, BearerToken, HasAuthentication, HasHeader},
	ApiEndpoint,
	ErrorType,
	RequestUserData,
};
use rustis::{client::Client as RedisClient, commands::StringCommands};
use time::OffsetDateTime;
use tower::{Layer, Service};

use crate::{models::access_token_data::AccessTokenData, prelude::*, utils::constants};

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
{
	endpoint: PhantomData<E>,
	client_type: ClientType,
}

impl<E> AuthenticationLayer<E>
where
	E: ApiEndpoint,
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
	A: HasAuthentication,
	E: ApiEndpoint,
{
	inner: S,
	client_type: ClientType,
	authenticator: PhantomData<A>,
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AppRequest<'a, E>> for AuthenticationService<AppAuthentication<E>, E, S>
where
	E: ApiEndpoint,
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

	#[instrument(skip(self, req))]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		let client_type = self.client_type;
		async move {
			trace!("Authenticating request");
			let BearerToken(token) = req.request.headers.get_header();
			let token = token.as_str();

			let user_data = match client_type {
				ClientType::ApiToken => {
					trace!("Parsing authentication header as an API token");
					let (refresh_token, login_id) = token.split_once('.').ok_or_else(|| {
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

					let login_id = Uuid::parse_str(login_id).map_err(|err| {
						warn!("Invalid API token provided: {}", token);
						warn!("Cannot parse loginId `{}` as UUID: {}", login_id, err);
						ErrorType::MalformedApiToken
					})?;

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

					if let Some(nbf) = token.token_nbf {
						if OffsetDateTime::now_utc() < nbf {
							info!("API token is not valid yet");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					}

					if let Some(exp) = token.token_exp {
						if OffsetDateTime::now_utc() > exp {
							info!("API token has expired");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					}

					if let Some(revoked) = token.revoked {
						if OffsetDateTime::now_utc() > revoked {
							info!("API token has been revoked");
							return Err(ErrorType::AuthorizationTokenInvalid);
						}
					}

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
					.map_err(|err| ErrorType::server_error(err.to_string()))?
					.verify_password(refresh_token.as_bytes(), &password_hash)
					.is_ok();

					if !success {
						warn!("API token has invalid refresh token");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}

					let permissions =
						get_permissions_for_login_id(req.database, req.redis, &login_id).await?;

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
								aud: _,
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

							validation
						},
					)
					.map_err(|err| {
						warn!("Invalid JWT provided: {}", err);
						return ErrorType::MalformedAccessToken;
					})?;

					if iss != constants::JWT_ISSUER {
						warn!("Invalid JWT issuer: {}", iss);
						return Err(ErrorType::MalformedAccessToken);
					}

					// The token should have been issued within the last `REFRESH_TOKEN_VALIDITY`
					// duration
					if OffsetDateTime::now_utc().sub(
						jti.get_timestamp()
							.ok_or_else(|| ErrorType::MalformedAccessToken)?,
					) > AccessTokenData::REFRESH_TOKEN_VALIDITY
					{
						warn!("JWT is too old");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}

					if OffsetDateTime::now_utc() < nbf {
						warn!("JWT is not valid yet");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}

					if OffsetDateTime::now_utc() > exp {
						warn!("JWT has expired");
						return Err(ErrorType::AuthorizationTokenInvalid);
					}

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
						WHERE
							user_login.login_id = $1 AND
							user_login.login_type = 'web_login';
						"#,
						sub as _
					}
					.fetch_optional(&mut **req.database)
					.await?
					else {
						warn!("API token not found");
						// No specific error for API token not found, since we don't want to leak
						// information about whether a loginId is valid or if it's expired
						return Err(ErrorType::AuthorizationTokenInvalid);
					};

					let permissions =
						get_permissions_for_login_id(req.database, req.redis, &sub).await?;

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
	A: HasAuthentication,
	E: ApiEndpoint,
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

async fn get_permissions_for_login_id(
	_db_connection: &mut DatabaseConnection,
	redis_connection: &mut RedisClient,
	login_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, ErrorType> {
	let redis_data: Option<String> = redis_connection
		.get(redis::keys::permission_for_login_id(login_id))
		.await?;
	if let Some(data) = redis_data {
		return Ok(serde_json::from_str(data.as_str())?);
	}

	todo!("Fetch from db")
}
