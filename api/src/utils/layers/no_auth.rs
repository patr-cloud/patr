use std::{
	marker::PhantomData,
	task::{Context, Poll},
};

use tower::{Layer, Service};

use crate::prelude::*;

#[derive(Clone, Debug, Copy)]
pub struct NoAuthMiddlewareLayer<E>
where
	E: ApiEndpoint,
{
	phantom: PhantomData<E>,
}

impl<E> NoAuthMiddlewareLayer<E>
where
	E: ApiEndpoint,
{
	pub const fn new() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<E> Default for NoAuthMiddlewareLayer<E>
where
	E: ApiEndpoint,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<S, E> Layer<S> for NoAuthMiddlewareLayer<E>
where
	S: Service<AppRequest<E>>,
	E: ApiEndpoint,
{
	type Service = NoAuthMiddleware<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		NoAuthMiddleware {
			inner,
			phantom: PhantomData,
		}
	}
}

#[derive(Clone, Debug)]
pub struct NoAuthMiddleware<S, E>
where
	S: Service<AppRequest<E>>,
	E: ApiEndpoint,
{
	inner: S,
	phantom: PhantomData<E>,
}

impl<S, E> Service<AppRequest<E>> for NoAuthMiddleware<S, E>
where
	E: ApiEndpoint,
	S: Service<AppRequest<E>>,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = S::Future;

	fn poll_ready(
		&mut self,
		cx: &mut Context<'_>,
	) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: AppRequest<E>) -> Self::Future {
		self.inner.call(req)
	}
}
