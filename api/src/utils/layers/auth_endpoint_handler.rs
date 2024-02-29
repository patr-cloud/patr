use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::prelude::*;

/// A trait that is implemented for all functions and closures that are used to
/// handle endpoints that require authentication. This trait is used to
/// implement the [`Layer`] trait for anything that takes an
/// [`AuthenticatedAppRequest`] and returns a `Future` that resolves to a
/// [`Result<AppResponse<E>, ErrorType>`].
pub trait AuthEndpointHandler<'req, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The `Future` type that is returned by the handler.
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send;

	/// Call the handler with the given authenticated request.
	fn call(self, req: AuthenticatedAppRequest<'req, E>) -> Self::Future;
}

impl<'req, F, Fut, E> AuthEndpointHandler<'req, E> for F
where
	F: FnOnce(AuthenticatedAppRequest<'req, E>) -> Fut + Send,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Future = Fut;

	fn call(self, req: AuthenticatedAppRequest<'req, E>) -> Self::Future {
		self(req)
	}
}

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_auth_endpoint`][1], and you should not need to use this
/// directly.
///
/// [1]: crate::utils::RouterExt::mount_auth_endpoint
pub struct AuthEndpointLayer<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<H, E> AuthEndpointLayer<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`AuthEndpointLayer`] with the given
	/// function or closure.
	pub fn new(handler: H) -> Self {
		Self {
			handler,
			endpoint: PhantomData,
		}
	}
}

impl<S, H, E> Layer<S> for AuthEndpointLayer<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Service = AuthEndpointService<H, E>;

	fn layer(&self, _: S) -> Self::Service {
		AuthEndpointService {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

impl<H, E> Clone for AuthEndpointLayer<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

/// The [`Service`] used by the [`AuthEndpointLayer`]. Ideally, this will
/// automatically be done by [`RouterExt::mount_auth_endpoint`][1], and you
/// should not need to use this directly.
///
/// [1]: crate::utils::RouterExt::mount_auth_endpoint
pub struct AuthEndpointService<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<H, E> AuthEndpointService<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`AuthEndpointService`] with the given
	/// function or closure.
	pub fn new(handler: H) -> Self {
		Self {
			handler,
			endpoint: PhantomData,
		}
	}
}

impl<'req, H, E> Service<AuthenticatedAppRequest<'req, E>> for AuthEndpointService<H, E>
where
	for<'anon> H: AuthEndpointHandler<'anon, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<AppResponse<E>, Self::Error>> + Send;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	#[instrument(skip(self, req), name = "AuthEndpointService")]
	fn call(&mut self, req: AuthenticatedAppRequest<'req, E>) -> Self::Future {
		self.handler.clone().call(req)
	}
}

impl<H, E> Clone for AuthEndpointService<H, E>
where
	for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}
