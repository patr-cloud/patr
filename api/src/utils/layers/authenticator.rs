use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::utils::{AppAuthentication, BearerToken, HasHeader};
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{models::permissions, prelude::*};

/// The type of client used for a request. This is used to determine
/// which authentication method to use, based on if the API call is made by our
/// web dashboard or by a third party application using the API token. This is
/// required because some endpoints are only accessible by the web dashboard,
/// and some are only accessible by third party applications. For example, you
/// cannot change your password, or create a new user using the API token, but
/// you can do so using the web dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
	/// The request is authenticated using a JWT from the web dashboard
	WebDashboard,
	/// The request is authenticated using an API token
	ApiToken,
}

/// The [`tower::Layer`] used to authenticate requests. This will parse the
/// [`BearerToken`] header and verify it against the database. If the token is
/// valid, the [`RequestUserData`][1] will be added to the request. All
/// subsequent underlying layers will recieve an [`AuthenticatedAppRequest`]
/// with the appropriate [`RequestUserData`][1] filled.
///
/// [1]: ::models::RequestUserData
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
	type Service = AuthenticationService<E, S>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthenticationService {
			inner,
			client_type: self.client_type,
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
pub struct AuthenticationService<E, S>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called after the request is authenticated
	inner: S,
	/// The type of client that is allowed to make the request
	client_type: ClientType,
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

	#[instrument(skip(self, req), name = "AuthenticatorService")]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		let allowed_client_type = self.client_type;
		async move {
			trace!("Authenticating request");
			let BearerToken(token) = req.request.headers.get_header();
			let token = token.token();

			let user_data = permissions::get_user_data_for_token(
				req.database,
				req.redis,
				allowed_client_type,
				&req.config,
				req.client_ip,
				token,
			)
			.await?;

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

impl<E, S> Clone for AuthenticationService<E, S>
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
			endpoint: PhantomData,
		}
	}
}
