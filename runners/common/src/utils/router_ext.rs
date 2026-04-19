use axum::{
	Router,
	routing::{MethodFilter, MethodRouter},
};
use axum_extra::routing::TypedPath;
use models::{
	api::ApiEndpoint,
	utils::{ClientType, HasHeader, NoAuthentication},
};
use preprocess::Preprocessable;
use tower::ServiceBuilder;

use crate::{
	prelude::*,
	utils::layers::{
		AuthenticationLayer,
		DataStoreConnectionLayer,
		EndpointHandler,
		EndpointLayer,
		PreprocessLayer,
		RequestParserLayer,
	},
};

/// Extension trait for the `Router` type.
///
/// This trait provides additional methods to mount API endpoints directly along
/// with the required request parser, and endpoint handler, using tower layers.
pub trait RouterExt<S>
where
	S: Clone + Send + Sync + 'static,
{
	/// Mount an API endpoint directly along with the required request parser,
	/// and endpoint handler, using tower layers.
	#[track_caller]
	#[must_use]
	fn mount_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		R: RunnerExecutor + Send + 'static;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	#[must_use]
	fn mount_auth_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		R: RunnerExecutor + Send + 'static;
}

impl<S> RouterExt<S> for Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	#[instrument(skip_all)]
	fn mount_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		R: RunnerExecutor + Send + 'static,
	{
		// Setup the layers for the backend
		if <E as ApiEndpoint>::ALLOWED_CLIENT_TYPES.contains(&ClientType::ApiToken) ||
			cfg!(debug_assertions)
		{
			self.route(
				<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
				MethodRouter::<S>::new()
					.on(
						MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
						async || {},
					)
					.layer(
						ServiceBuilder::new()
							// .layer(todo!("Add rate limiter checker middleware here")),
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(EndpointLayer::new(handler.clone())),
					),
			)
		} else {
			self
		}
	}

	#[instrument(skip_all)]
	fn mount_auth_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		R: RunnerExecutor + Send + 'static,
	{
		// Setup the layers for the backend
		if <E as ApiEndpoint>::ALLOWED_CLIENT_TYPES.contains(&ClientType::ApiToken) ||
			cfg!(debug_assertions)
		{
			self.route(
				<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
				MethodRouter::<S>::new()
					.on(
						MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
						async || {},
					)
					.layer(
						ServiceBuilder::new()
							// .layer(todo!("Add rate limiter checker middleware here")),
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(AuthenticationLayer::new())
							.layer(EndpointLayer::new(handler.clone())),
					),
			)
		} else {
			self
		}
	}
}
