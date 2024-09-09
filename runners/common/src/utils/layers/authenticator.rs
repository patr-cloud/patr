use std::{
	future::Future,
	marker::PhantomData,
	ops::Sub,
	task::{Context, Poll},
};

use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::utils::HasHeader;
use preprocess::Preprocessable;
use time::OffsetDateTime;
use tower::{Layer, Service};

use crate::{app::AppRequest, prelude::*, utils::access_token_data::AccessTokenData};

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
/// underlying layers will recieve an [`AppRequest`] with the
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
	for<'a> S: Service<AppRequest<'a, E>>,
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
	for<'b> S: Service<AppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
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

			match client_type {
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

					let _refresh_token = Uuid::parse_str(refresh_token).map_err(|err| {
						warn!("Invalid API token provided: {}", token);
						warn!(
							"Cannot parse refresh token `{}` as UUID: {}",
							refresh_token, err
						);
						ErrorType::MalformedApiToken
					})?;
					trace!("Refresh token parsed as UUID");

					let _login_id = Uuid::parse_str(login_id).map_err(|err| {
						warn!("Invalid API token provided: {}", token);
						warn!("Cannot parse loginId `{}` as UUID: {}", login_id, err);
						ErrorType::MalformedApiToken
					})?;
					trace!("Login ID parsed as UUID");

					todo!("Add the API token authenticator");
				}
				ClientType::WebDashboard => {
					trace!("Parsing authentication header as a JWT");

					let TokenData {
						header: _,
						claims:
							AccessTokenData {
								iss,
								sub: _,
								aud: _,
								exp,
								nbf,
								iat: _,
								jti,
							},
					} = jsonwebtoken::decode(
						token,
						// TODO: Change this to use the JWT secret from the config
						&DecodingKey::from_secret(b"keyboard cat"),
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
				}
			};

			inner.call(req).await
		}
	}
}

impl<A, E, S> Clone for AuthenticationService<A, E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
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
