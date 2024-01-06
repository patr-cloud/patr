use std::net::IpAddr;

use axum::{
	body::Body,
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{
	utils::{AppAuthentication, BearerToken, HasHeader, NoAuthentication},
	ApiEndpoint,
	ApiRequest,
	AppResponse,
	ErrorType,
};
use tower::{
	util::{BoxCloneService, BoxLayer},
	ServiceBuilder,
};

use super::layers::{AuthenticationLayer, ClientType, RequestParserLayer};
use crate::{
	prelude::AppState,
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
	fn mount_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		E::RequestHeaders: HasHeader<BearerToken>;
}

impl<S> RouterExt<S> for Router<S, Body>
where
	S: Clone + Send + Sync + 'static,
{
	fn mount_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
	{
		let mut registry = frontend::utils::API_CALL_REGISTRY
			.write()
			.expect("API call registry poisoned");

		if registry.get().is_none() {
			registry
				.set(Default::default())
				.map_err(|_| ())
				.expect("registry already set when it is expected to not be set");
		}

		registry
			.get_mut()
			.expect("registry should be initialized by now")
			.entry(E::METHOD)
			.or_default()
			.insert(
				<E::RequestPath as TypedPath>::PATH,
				Box::new(BoxLayer::<
					BoxCloneService<(ApiRequest<E>, IpAddr), AppResponse<E>, ErrorType>,
					(ApiRequest<E>, IpAddr),
					AppResponse<E>,
					ErrorType,
				>::new(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(DataStoreConnectionLayer::<E>::with_state(state.clone()))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						.layer(EndpointLayer::new(handler.clone())),
				)),
			)
			.expect("API endpoint already registered");

		// Setup the layers for the backend
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(RequestParserLayer::new())
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						.layer(EndpointLayer::new(handler)),
				),
		)
	}

	fn mount_auth_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		E::RequestHeaders: HasHeader<BearerToken>,
	{
		let mut registry = frontend::utils::API_CALL_REGISTRY
			.write()
			.expect("API call registry poisoned");

		if registry.get().is_none() {
			registry
				.set(Default::default())
				.map_err(|_| ())
				.expect("registry already set when it is expected to not be set");
		}

		registry
			.get_mut()
			.expect("registry should be initialized by now")
			.entry(E::METHOD)
			.or_default()
			.insert(
				<E::RequestPath as TypedPath>::PATH,
				Box::new(BoxLayer::<
					BoxCloneService<(ApiRequest<E>, IpAddr), AppResponse<E>, ErrorType>,
					(ApiRequest<E>, IpAddr),
					AppResponse<E>,
					ErrorType,
				>::new(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						.layer(AuthenticationLayer::new(ClientType::WebDashboard))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						// .layer(todo!("Add audit logger middleware here"))
						.layer(AuthEndpointLayer::new(handler.clone())),
				)),
			)
			.expect("API endpoint already registered");

		// Setup the layers for the backend
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(
					ServiceBuilder::new()
						// .layer(todo!("Add rate limiter checker middleware here")),
						.layer(RequestParserLayer::new())
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						.layer(AuthenticationLayer::new(ClientType::ApiToken))
						// .layer(todo!("Add rate limiter value updater middleware here"))
						// .layer(todo!("Add audit logger middleware here"))
						.layer(AuthEndpointLayer::new(handler)),
				),
		)
	}
}
