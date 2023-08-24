use std::marker::PhantomData;

use axum::{
	body::HttpBody,
	handler::Handler,
	http::Request,
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{utils::ApiRequest, ApiEndpoint};

use crate::utils::{route_handler::EndpointHandler, LastElementIs};

/// Extension trait for axum Router to mount an API endpoint directly along with
/// the required request parser, Rate limiter, Audit logger and Auth
/// middlewares, using tower layers.
pub trait RouterExt<S, B>
where
	B: HttpBody + Send + 'static,
	S: Clone + Send + Sync + 'static,
{
	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_endpoint<E, Params, H>(self, handler: H) -> Self
	where
		H: EndpointHandler<Params, S, B, E> + Clone + Send + 'static,
		Params: LastElementIs<ApiRequest<E>> + Send + 'static,
		E: ApiEndpoint;
}

impl<S, B> RouterExt<S, B> for Router<S, B>
where
	B: HttpBody + Send + 'static,
	S: Clone + Send + Sync + 'static,
{
	#[track_caller]
	fn mount_endpoint<E, Params, H>(self, handler: H) -> Self
	where
		H: EndpointHandler<Params, S, B, E> + Clone + Send + 'static,
		Params: LastElementIs<ApiRequest<E>> + Send + 'static,
		E: ApiEndpoint,
	{
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S, B>::new().on(
				MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
				EndpointHandlerToAxumHandler {
					handler,
					params: PhantomData,
					state: PhantomData,
					body: PhantomData,
					endpoint: PhantomData,
				}
				.layer(todo!("Add audit logger middleware here"))
				.layer(todo!("Add auth middleware here"))
				.layer(todo!("Add request parser middleware here"))
				.layer(todo!("Add rate limiter middleware here")),
			),
		)
	}
}

struct EndpointHandlerToAxumHandler<H, Params, S, B, E>
where
	H: EndpointHandler<Params, S, B, E> + Clone + Send + 'static,
	S: Clone + Send + Sync + 'static,
	B: HttpBody + Send + 'static,
	Params: LastElementIs<ApiRequest<E>> + Send + 'static,
	E: ApiEndpoint,
{
	handler: H,
	params: PhantomData<Params>,
	state: PhantomData<S>,
	body: PhantomData<B>,
	endpoint: PhantomData<E>,
}

impl<H, Params, S, B, E> Clone
	for EndpointHandlerToAxumHandler<H, Params, S, B, E>
where
	H: EndpointHandler<Params, S, B, E> + Clone + Send + 'static,
	S: Clone + Send + Sync + 'static,
	B: HttpBody + Send + 'static,
	Params: LastElementIs<ApiRequest<E>> + Send + 'static,
	E: ApiEndpoint,
{
	fn clone(&self) -> Self {
		Self {
			handler: self.handler.clone(),
			params: PhantomData,
			state: PhantomData,
			body: PhantomData,
			endpoint: PhantomData,
		}
	}
}

impl<H, Params, S, B, E> Handler<Params, S, B>
	for EndpointHandlerToAxumHandler<H, Params, S, B, E>
where
	H: EndpointHandler<Params, S, B, E> + Send + Clone + 'static,
	S: Clone + Send + Sync + 'static,
	B: HttpBody + Send + 'static,
	Params: LastElementIs<ApiRequest<E>> + Send + 'static,
	E: ApiEndpoint,
{
	type Future = H::Future;

	fn call(self, req: Request<B>, state: S) -> Self::Future {
		self.handler.call(req, state)
	}
}
