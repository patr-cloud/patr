use std::{
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use models::{ApiEndpoint, ErrorType, utils::{False, True}};
use tower::{Layer, Service};

use crate::{app::AppResponse, prelude::{AppRequest, AuthenticatedAppRequest}};

pub trait EndpointHandler<E>
where
	E: ApiEndpoint,
{
	type Future: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static;
	type Request<'a>;

	fn call(self, req: Self::Request<'_>) -> Self::Future;
}

impl<F, Fut, E> EndpointHandler<E> for F
where
	F: FnOnce(AppRequest<'_, E>) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static,
	E: ApiEndpoint,
	E::AUTHD: Into<False>,
{
	type Future = Fut;
	type Request<'a> = AppRequest<'a, E>;

	fn call(self, req: Self::Request<'_>) -> Self::Future {
		self(req)
	}
}

impl<F, Fut, E> EndpointHandler<E> for F
where
	F: FnOnce(AuthenticatedAppRequest<'_, E>) -> Fut + Clone + Send + 'static,
	Fut: Future<Output = Result<AppResponse<E>, ErrorType>> + Send + 'static,
	E: ApiEndpoint,
	E::AUTHD: Into<True>,
{
	type Future = Fut;
	type Request<'a> = AuthenticatedAppRequest<'a, E>;

	fn call(self, req: Self::Request<'_>) -> Self::Future {
		self(req)
	}
}

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

	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
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
