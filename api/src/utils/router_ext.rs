use axum::{
	Router,
	routing::{MethodFilter, MethodRouter},
};
use axum_extra::routing::TypedPath;
use models::utils::{AppAuthentication, BearerToken, HasHeader, NoAuthentication};
use preprocess::Preprocessable;
use tower::ServiceBuilder;

use super::layers::{
	AuthenticationLayer,
	AuthorizationLayer,
	ClientType,
	PreprocessLayer,
	RequestParserLayer,
	UserAgentValidationLayer,
};
use crate::{
	prelude::*,
	utils::layers::{
		AuthEndpointHandler,
		AuthEndpointLayer,
		DataStoreConnectionLayer,
		EndpointHandler,
		EndpointLayer,
	},
};

/// Extension trait for axum Router to mount an API endpoint directly along with
/// the required request parser, Rate limiter, Audit logger and Auth
/// middlewares, using tower layers.
pub trait RouterExt<S>
where
	S: Clone + Send + Sync + 'static,
{
	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter using tower layers.
	#[track_caller]
	fn mount_endpoint<E, H>(
		self,
		handler: H,
		state: &AppState,
		allowed_client_type: ClientType,
	) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H>(
		self,
		handler: H,
		state: &AppState,
		allowed_client_type: ClientType,
	) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>;
}

impl<S> RouterExt<S> for Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	#[instrument(skip_all)]
	fn mount_endpoint<E, H>(
		self,
		handler: H,
		state: &AppState,
		allowed_client_type: ClientType,
	) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
	{
		// Setup the layers for the backend

		if allowed_client_type == ClientType::ApiToken && !<E as ApiEndpoint>::API_ALLOWED {
			// If the client type is API token and the endpoint is not allowed for API
			// tokens, skip mounting the endpoint
			self
		} else {
			// For all other cases, mount the endpoint
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
							// .layer(todo!("Add rate limiter value updater middleware here"))
							.layer(PreprocessLayer::new())
							.layer(UserAgentValidationLayer::new())
							.layer(EndpointLayer::new(handler)),
					),
			)
		}
	}

	#[instrument(skip_all)]
	fn mount_auth_endpoint<E, H>(
		self,
		handler: H,
		state: &AppState,
		allowed_client_type: ClientType,
	) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
	{
		// Setup the layers for the backend

		if allowed_client_type == ClientType::ApiToken && !<E as ApiEndpoint>::API_ALLOWED {
			// If the client type is API token and the endpoint is not allowed for API
			// tokens, skip mounting the endpoint
			self
		} else {
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
							.layer(UserAgentValidationLayer::new())
							.layer(AuthenticationLayer::new(allowed_client_type))
							.layer(AuthorizationLayer::new())
							// .layer(todo!("Add rate limiter value updater middleware here"))
							// .layer(todo!("Add audit logger middleware here"))
							.layer(AuthEndpointLayer::new(handler)),
					),
			)
		}
	}
}
