use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::{
	utils::{AppAuthentication, BearerToken, HasAuthentication, HasHeader},
	ApiEndpoint,
	ErrorType,
};
use tower::{Layer, Service};

use crate::{
	app::AppResponse,
	prelude::{AppRequest, AuthenticatedAppRequest},
};

pub struct AuthenticationLayer<E>
where
	E: ApiEndpoint,
{
	endpoint: PhantomData<E>,
}

impl<E> AuthenticationLayer<E>
where
	E: ApiEndpoint,
{
	pub fn new() -> Self {
		Self {
			endpoint: PhantomData,
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
		}
	}
}

pub struct AuthenticationService<A, E, S>
where
	A: HasAuthentication,
	E: ApiEndpoint,
{
	inner: S,
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
	type Response = AppResponse<E>;
	type Error = ErrorType;
	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			let bearer_token = req.request.headers.get_header();
			let req = todo!();
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
			authenticator: PhantomData,
			endpoint: PhantomData,
		}
	}
}
