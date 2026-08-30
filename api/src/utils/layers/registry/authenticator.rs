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

use models::utils::HasHeader;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{models::permissions, routes::registry_patr_cloud::prelude::*};

/// Layer that authenticates registry requests using API tokens.
///
/// This layer:
/// 1. Extracts the Authorization header (Bearer token)
/// 2. Validates the token as an API token (format: patrv1.{refresh_token}.{login_id})
/// 3. Verifies the token against the database
/// 4. Checks token expiration, revocation, and IP restrictions
/// 5. Loads user permissions from Redis cache or database
/// 6. Converts `RegistryRequestWithConnections` to `AuthenticatedRegistryRequest`
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

			let user_data = permissions::get_user_data_for_token(
				req.database,
				req.redis,
				ClientType::ApiToken,
				&req.config,
				req.client_ip,
				token,
			)
			.await
			.map_err(|err| {
				RegistryError::builder()
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.code(ErrorCode::Unsupported)
					.message(
						if cfg!(debug_assertions) {
							err.to_string()
						} else {
							"Authentication failed".to_string()
						},
					)
					.build()
			})?;

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
