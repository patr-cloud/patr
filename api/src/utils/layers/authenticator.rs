use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::utils::{AppAuthentication, BearerToken, HasHeader};
use preprocess::Preprocessable;
use tower::{Layer, Service};
use tracing::{Span, field::display};

use crate::{models::permissions, prelude::*};

/// The [`tower::Layer`] used to authenticate requests. This will parse the
/// [`BearerToken`] header and verify it against the database. If the token is
/// valid, the [`RequestUserData`][1] will be added to the request. All
/// subsequent underlying layers will recieve an [`AuthenticatedAppRequest`]
/// with the appropriate [`RequestUserData`][1] filled.
///
/// [1]: ::models::RequestUserData
pub struct AuthenticationLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<E> AuthenticationLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Helper function to initialize an authentication layer
	pub fn new() -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

impl<E> Default for AuthenticationLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<E, S> Layer<S> for AuthenticationLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'a> S: Service<AuthenticatedAppRequest<'a, E>>,
{
	type Service = AuthenticationService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthenticationService {
			inner,
			endpoint: PhantomData,
		}
	}
}

impl<E> Clone for AuthenticationLayer<E>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			endpoint: PhantomData,
		}
	}
}

/// The underlying service that runs when the [`AuthenticationLayer`] is used.
pub struct AuthenticationService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request is authenticated
	inner: S,
	/// The endpoint type that this layer will handle
	endpoint: PhantomData<E>,
}

impl<'a, E, S> Service<AppRequest<'a, E>> for AuthenticationService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
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

	#[instrument(skip(self, req), name = "AuthenticatorService", fields(
		patr.user_id,
		patr.login_id,
	))]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			trace!("Authenticating request");
			let BearerToken(token) = req.request.headers.get_header();
			let token = token.token();

			let user_data = permissions::get_user_data_for_token(
				req.database,
				req.redis,
				&req.state.config,
				req.client_ip,
				token,
			)
			.await?;

			if !<E as ApiEndpoint>::ALLOWED_CLIENT_TYPES.contains(&user_data.client_type) {
				return Err(ErrorType::Unauthorized);
			}

			Span::current().record("patr.user_id", display(user_data.id));
			Span::current().record("patr.login_id", display(user_data.login_id));

			let AppRequest {
				request,
				database,
				redis,
				client_ip,
				state,
			} = req;
			let req = AuthenticatedAppRequest {
				request,
				database,
				redis,
				client_ip,
				state,
				user_data,
			};
			inner.call(req).await
		}
	}
}

impl<E, S> Clone for AuthenticationService<E, S>
where
	E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	for<'b> S: Service<AuthenticatedAppRequest<'b, E>, Response = AppResponse<E>, Error = ErrorType>
		+ Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			endpoint: PhantomData,
		}
	}
}
