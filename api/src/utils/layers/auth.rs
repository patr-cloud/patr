use std::task::Context;

use models::{
	utils::{ApiRequest, Uuid},
	ApiEndpoint,
};
use tower::{Layer, Service};

use crate::prelude::AppRequest;

#[derive(Debug, Clone)]
pub enum AuthMiddlewareLayer<E>
where
	E: ApiEndpoint,
{
	PlainTokenAuthenticator,
	WorkspaceMembershipAuthenticator {
		workspace_extractor: fn(&ApiRequest<E>) -> Uuid,
	},
}

impl<S, E> Layer<S> for AuthMiddlewareLayer<E>
where
	for<'a> S: Service<AppRequest<'a, E>>,
	E: ApiEndpoint,
{
	type Service = AuthMiddleware<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthMiddleware {
			inner,
			auth_type: self.clone(),
		}
	}
}

pub struct AuthMiddleware<S, E>
where
	for<'a> S: Service<AppRequest<'a, E>>,
	E: ApiEndpoint,
{
	inner: S,
	auth_type: AuthMiddlewareLayer<E>,
}

impl<'a, S, E> Service<AppRequest<'a, E>> for AuthMiddleware<S, E>
where
	for<'b> S: Service<AppRequest<'b, E>>,
	E: ApiEndpoint,
{
	type Response = <S as Service<AppRequest<'a, E>>>::Response;
	type Error = <S as Service<AppRequest<'a, E>>>::Error;
	type Future = <S as Service<AppRequest<'a, E>>>::Future;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: AppRequest<'a, E>) -> Self::Future {
		todo!()
	}
}
