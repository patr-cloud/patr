use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::prelude::*;

/// A trait that is implemented for functions and closures that take a specific
/// request and returns an async fn that returns a response. This can be used to
/// mount endpoint handlers that respond to specific API endpoints as per the
/// [`ApiEndpoint`] trait.
pub trait EndpointHandler<'req, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The future returned by the endpoint handler.
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send;

	/// Call the endpoint handler with the given request.
	fn call(self, req: AppRequest<'req, E>) -> Self::Future;
}

impl<'req, F, Fut, E> EndpointHandler<'req, E> for F
where
	F: FnOnce(AppRequest<'req, E>) -> Fut,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Future = Fut;

	fn call(self, req: AppRequest<'req, E>) -> Self::Future {
		self(req)
	}
}

/// A [`tower::Layer`] that can be used mount the endpoint to the router.
/// Ideally, this will automatically be done by [`RouterExt::mount_endpoint`],
/// and you should not need to use this directly.
pub struct EndpointLayer<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<H, E> EndpointLayer<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`EndpointLayer`] with the given endpoint
	/// handler.
	pub fn new(handler: H) -> Self {
		Self {
			handler,
			endpoint: PhantomData,
		}
	}
}

impl<S, H, E> Layer<S> for EndpointLayer<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Service = EndpointService<H, E>;

	fn layer(&self, _: S) -> Self::Service {
		EndpointService {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

impl<H, E> Clone for EndpointLayer<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
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

/// A [`tower::Service`] that can be used mount the endpoint to the router.
pub struct EndpointService<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'req, H, E> Service<AppRequest<'req, E>> for EndpointService<H, E>
where
	for<'anon> H: EndpointHandler<'anon, E> + Clone + Send,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = ErrorType;
	type Response = AppResponse<E>;

	type Future = impl Future<Output = Result<AppResponse<E>, Self::Error>> + Send;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	#[instrument(skip(self, req), name = "EndpointService")]
	fn call(&mut self, req: AppRequest<'req, E>) -> Self::Future {
		trace!("Calling request handler");
		self.handler.clone().call(req)
	}
}

impl<H, E> Clone for EndpointService<H, E>
where
	for<'req> H: EndpointHandler<'req, E> + Clone + Send,
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
