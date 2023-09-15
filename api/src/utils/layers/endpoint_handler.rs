use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::{ApiEndpoint, ErrorType};
use tower::{Layer, Service};

use crate::prelude::*;

/// A trait that is implemented for functions and closures that take a specific
/// request and returns an async fn that returns a response. This can be used to
/// mount endpoint handlers that respond to specific API endpoints as per the
/// [`ApiEndpoint`] trait.
pub trait EndpointHandler<E>
where
	E: ApiEndpoint,
{
	/// The future returned by the endpoint handler.
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static;

	/// Call the endpoint handler with the given request.
	fn call<'a>(self, req: AppRequest<'a, E>) -> Self::Future;
}

impl<F, Fut, E> EndpointHandler<E> for F
where
	F: FnOnce(AppRequest<'_, E>) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static,
	E: ApiEndpoint,
{
	type Future = Fut;

	fn call(self, req: AppRequest<'_, E>) -> Self::Future {
		self(req)
	}
}

/// A [`tower::Layer`] that can be used mount the endpoint to the router.
/// Ideally, this will automatically be done by [`RouterExt::mount_endpoint`],
/// and you should not need to use this directly.
pub struct EndpointLayer<H, E>
where
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	handler: H,
	endpoint: PhantomData<E>,
}

impl<H, E> EndpointLayer<H, E>
where
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
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
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
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
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
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
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	handler: H,
	endpoint: PhantomData<E>,
}

impl<'a, H, E> Service<AppRequest<'a, E>> for EndpointService<H, E>
where
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	type Response = AppResponse<E>;
	type Error = ErrorType;
	type Future = H::Future;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	#[instrument(skip(self, req))]
	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		trace!("Calling request handler");
		self.handler.clone().call(req)
	}
}

impl<H, E> Clone for EndpointService<H, E>
where
	H: EndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}
