use axum::{
	Router,
	routing::{MethodFilter, MethodRouter},
};
use axum_extra::routing::TypedPath;
use headers::UserAgent;
use models::utils::{AppAuthentication, BearerToken, HasHeader, NoAuthentication};
use preprocess::Preprocessable;
use tower::ServiceBuilder;

use crate::{
	prelude::*,
	routes::registry_patr_cloud::prelude::RegistryEndpoint,
	utils::layers::{
		AuditLoggerLayer,
		AuthEndpointHandler,
		AuthEndpointLayer,
		AuthRateLimiterLayer,
		AuthenticationLayer,
		AuthorizationLayer,
		ClientType,
		DataStoreConnectionLayer,
		EndpointHandler,
		EndpointLayer,
		PreprocessLayer,
		RateLimiterLayer,
		RequestParserLayer,
		UserAgentValidationLayer,
		WebDashboardAuthCookieLayer,
		registry::{
			RegistryAuthenticationLayer,
			RegistryDataStoreConnectionLayer,
			RegistryEndpointHandler,
			RegistryEndpointLayer,
			RegistryPreprocessLayer,
			RegistryRateLimiterLayer,
			RegistryRequestParserLayer,
		},
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
		E::RequestHeaders: HasHeader<BearerToken> + HasHeader<UserAgent>;

	/// Mount a registry endpoint. This sets up the necessary layers for request
	/// parsing and data store connection.
	#[track_caller]
	fn mount_registry_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: RegistryEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: RegistryEndpoint + Sync,
		<E::RequestPath as Preprocessable>::Processed: Send,
		<E::RequestQuery as Preprocessable>::Processed: Send,
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
					.route_layer(
						ServiceBuilder::new()
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(RateLimiterLayer::new())
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
		E::RequestHeaders: HasHeader<BearerToken> + HasHeader<UserAgent>,
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
					.route_layer(
						ServiceBuilder::new()
							.option_layer(
								if allowed_client_type == ClientType::WebDashboard {
									// For web dashboard, we need to extract the
									// auth state cookie and set that as the
									// Bearer token so that the
									// AuthenticationLayer can pick it up
									Some(WebDashboardAuthCookieLayer::new())
								} else {
									None
								},
							)
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(UserAgentValidationLayer::new())
							.layer(AuthenticationLayer::new(allowed_client_type))
							.layer(AuthorizationLayer::new())
							.layer(AuthRateLimiterLayer::new())
							.layer(AuditLoggerLayer::new())
							.layer(AuthEndpointLayer::new(handler)),
					),
			)
		}
	}

	#[instrument(skip_all)]
	fn mount_registry_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: RegistryEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: RegistryEndpoint + Sync,
		<E::RequestPath as Preprocessable>::Processed: Send,
		<E::RequestQuery as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
	{
		self.route(
			<<E as RegistryEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S>::new()
				.on(
					MethodFilter::try_from(<E as RegistryEndpoint>::METHOD).unwrap(),
					async || {},
				)
				.route_layer(
					ServiceBuilder::new()
						.layer(RegistryRequestParserLayer::with_state(state.clone()))
						.layer(RegistryDataStoreConnectionLayer::with_state(state.clone()))
						.layer(RegistryPreprocessLayer::new())
						.layer(RegistryAuthenticationLayer::new())
						.layer(RegistryRateLimiterLayer::new())
						// .layer(todo!("Add audit logger middleware here"))
						.layer(RegistryEndpointLayer::new(handler)),
				),
		)
	}
}
