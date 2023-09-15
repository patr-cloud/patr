use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::{ApiEndpoint, ErrorType};
use tower::{Layer, Service};

use crate::{app::AppResponse, prelude::AuthenticatedAppRequest};

/// A trait that is implemented for all functions and closures that are used to
/// handle endpoints that require authentication. This trait is used to
/// implement the [`Layer`] trait for anything that takes an
/// [`AuthenticatedAppRequest`] and returns a `Future` that resolves to a
/// [`Result<AppResponse<E>, ErrorType>>`].
pub trait AuthEndpointHandler<E>
where
	E: ApiEndpoint,
{
	/// The `Future` type that is returned by the handler.
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static;

	/// Call the handler with the given authenticated request.
	fn call<'a>(self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future;
}

impl<F, Fut, E> AuthEndpointHandler<E> for F
where
	F: FnOnce(AuthenticatedAppRequest<'_, E>) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static,
	E: ApiEndpoint,
{
	type Future = Fut;

	fn call(self, req: AuthenticatedAppRequest<'_, E>) -> Self::Future {
		self(req)
	}
}

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_auth_endpoint`], and you should not need to use this
/// directly.
pub struct AuthEndpointLayer<H, E>
where
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	handler: H,
	endpoint: PhantomData<E>,
}

impl<H, E> AuthEndpointLayer<H, E>
where
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
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
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
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
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}

/// The [`Service`] used by the [`AuthEndpointLayer`]. Ideally, this will
/// automatically be done by [`RouterExt::mount_auth_endpoint`], and you should
/// not need to use this directly.
pub struct AuthEndpointService<H, E>
where
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	handler: H,
	endpoint: PhantomData<E>,
}

impl<'a, H, E> Service<AuthenticatedAppRequest<'a, E>> for AuthEndpointService<H, E>
where
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	type Error = ErrorType;
	type Future = H::Future;
	type Response = AppResponse<E>;

	fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: AuthenticatedAppRequest<'a, E>) -> Self::Future {
		self.handler.clone().call(req)
	}
}

impl<H, E> Clone for AuthEndpointService<H, E>
where
	H: AuthEndpointHandler<E> + Clone + Send + 'static,
	E: ApiEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			endpoint: PhantomData,
		}
	}
}
