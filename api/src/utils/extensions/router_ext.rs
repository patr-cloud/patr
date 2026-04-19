use axum::{
	Router,
	routing::{MethodFilter, MethodRouter},
};
use axum_extra::routing::TypedPath;
use headers::UserAgent;
use models::utils::{ActorClientType, AppAuthentication, BearerToken, HasHeader, NoAuthentication};
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
		host_client_types: &[ActorClientType],
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
		host_client_types: &[ActorClientType],
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
		host_client_types: &[ActorClientType],
	) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
	{
		// Setup the layers for the backend

		// No client the host serves may call this endpoint: nothing to mount.
		if !<E as ApiEndpoint>::ALLOWED_CLIENT_TYPES
			.iter()
			.any(|client_type| host_client_types.contains(client_type))
		{
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
		host_client_types: &[ActorClientType],
	) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken> + HasHeader<UserAgent>,
	{
		// Setup the layers for the backend

		// Who this route actually responds to here: the clients the endpoint
		// permits that the host also serves. Empty means it has no callers on
		// this host and isn't mounted at all; otherwise it is the one list the
		// authenticator checks.
		let served_client_types = <E as ApiEndpoint>::ALLOWED_CLIENT_TYPES
			.iter()
			.copied()
			.filter(|client_type| host_client_types.contains(client_type))
			.collect::<Vec<_>>();

		if served_client_types.is_empty() {
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
							// The cookie-to-Bearer shim is only needed if a
							// dashboard session can actually reach this route here.
							.option_layer(
								served_client_types
									.contains(&ActorClientType::WebDashboard)
									.then(WebDashboardAuthCookieLayer::new),
							)
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(UserAgentValidationLayer::new())
							.layer(AuthenticationLayer::new(served_client_types))
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
