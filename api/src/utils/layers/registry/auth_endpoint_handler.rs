/// Registry endpoint handler layers.
///
/// This module contains layers for handling registry endpoints,
/// both authenticated and non-authenticated.
///
/// These traits and layers allow handlers to be mounted to the router
/// and called with the appropriate request types.
use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::routes::registry_patr_cloud::prelude::*;

/// A trait that is implemented for functions and closures that take an
/// [`AuthenticatedRegistryRequest`] and return a future that resolves to a
/// [`Result<RegistryResponse<E>, RegistryError>`].
///
/// This trait is used to mount authenticated registry endpoint handlers.
pub trait AuthRegistryEndpointHandler<'req, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The future returned by the endpoint handler.
	type Future: Future<Output = Result<RegistryResponse<E>, RegistryError>> + Send;

	/// Call the endpoint handler with the given authenticated request.
	fn call(self, req: AuthenticatedRegistryAppRequest<'req, E>) -> Self::Future;
}

impl<'req, F, Fut, E> AuthRegistryEndpointHandler<'req, E> for F
where
	F: FnOnce(AuthenticatedRegistryAppRequest<'req, E>) -> Fut,
	Fut: Future<Output = Result<RegistryResponse<E>, RegistryError>> + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Future = Fut;

	fn call(self, req: AuthenticatedRegistryAppRequest<'req, E>) -> Self::Future {
		self(req)
	}
}

/// A [`tower::Layer`] that can be used to mount authenticated registry
/// endpoints to the router.
///
/// This layer wraps a handler function and converts it into a Tower service
/// that can process [`AuthenticatedRegistryAppRequest`] objects and return HTTP
/// responses.
///
/// Ideally, this will automatically be done by
/// [`RegistryRouterExt::mount_auth_registry_endpoint`], and you should not
/// need to use this directly.
pub struct AuthRegistryEndpointLayer<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this layer will handle.
	endpoint: PhantomData<E>,
}

impl<H, E> AuthRegistryEndpointLayer<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Create a new authenticated registry endpoint layer with the given
	/// handler.
	pub fn new(handler: H) -> Self {
		Self {
			handler,
			endpoint: PhantomData,
		}
	}
}

impl<S, H, E> Layer<S> for AuthRegistryEndpointLayer<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Service = AuthRegistryEndpointService<H, E>;

	fn layer(&self, _: S) -> Self::Service {
		AuthRegistryEndpointService {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

impl<H, E> Clone for AuthRegistryEndpointLayer<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

/// A [`tower::Service`] that handles authenticated registry endpoints.
///
/// This service calls the handler function with an
/// [`AuthenticatedRegistryAppRequest`] and converts the result into an HTTP
/// response, handling errors by converting them to OCI-compliant error
/// responses.
pub struct AuthRegistryEndpointService<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// The function or closure that will be used to handle the endpoint.
	handler: H,
	/// The endpoint type that this service will handle.
	endpoint: PhantomData<E>,
}

impl<'req, H, E> Service<AuthenticatedRegistryAppRequest<'req, E>>
	for AuthRegistryEndpointService<H, E>
where
	for<'anon> H: AuthRegistryEndpointHandler<'anon, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Error = RegistryError;
	type Response = RegistryResponse<E>;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>> + Send;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	#[instrument(skip(self, req), name = "AuthRegistryEndpointService")]
	fn call(&mut self, req: AuthenticatedRegistryAppRequest<'req, E>) -> Self::Future {
		trace!("Calling authenticated registry endpoint handler");
		self.handler.clone().call(req)
	}
}

impl<H, E> Clone for AuthRegistryEndpointService<H, E>
where
	for<'req> H: AuthRegistryEndpointHandler<'req, E> + Clone + Send,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}
