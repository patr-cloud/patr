use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::{ApiEndpoint, ErrorType};
use tower::{Layer, Service};

use crate::{app::AppResponse, prelude::AuthenticatedAppRequest};

pub trait AuthEndpointHandler<E>
where
	E: ApiEndpoint,
{
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static;

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
	type Response = AppResponse<E>;
	type Error = ErrorType;
	type Future = H::Future;

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
