use std::{any::Any, future::Future};

use axum::{
	body::Body,
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{
	utils::{AppAuthentication, BearerToken, HasHeader, NoAuthentication},
	ApiEndpoint,
	ErrorType,
};
use tower::{
	util::{BoxCloneService, BoxLayer},
	ServiceBuilder,
	ServiceExt,
};

use super::layers::{AuthenticationLayer, ClientType};
use crate::{
	prelude::AppState,
	utils::layers::{
		AuthEndpointHandler,
		AuthEndpointLayer,
		EndpointHandler,
		EndpointLayer,
		RequestParserLayer,
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
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
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
		// All the layers needed
		let layer_builder = |state, handler| {
			ServiceBuilder::new()
				// .layer(todo!("Add rate limiter checker middleware here")),
				.layer(RequestParserLayer::with_state(state))
				// .layer(todo!("Add rate limiter value updater middleware here"))
				.layer(EndpointLayer::new(handler))
		};

		// Get the index for the current layer
		let index = API_CALL_REGISTRY_INDEX
			.write()
			.expect("API call registry index poisoned and cannot be opened for writing")
			.len();

		// Add the index and layer to the index and layer registry
		frontend::utils::API_CALL_REGISTRY_INDEX
			.write()
			.expect("API call registry index poisoned and cannot be opened for writing")
			.insert(
				format!(
					"{} {}",
					<E as ApiEndpoint>::METHOD,
					<E::RequestPath as TypedPath>::PATH
				),
				index,
			);

		// Setup the layers for the frontend
		frontend::utils::API_CALL_REGISTRY
			.write()
			.expect("API call registry poisoned and cannot be opened for writing")
			.push(layer_builder(state.clone(), handler.clone()));

		// Setup the layers for the backend
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(layer_builder(state.clone(), handler)),
		)
	}

	fn mount_auth_endpoint<E, H>(self, handler: H, state: &AppState) -> Self
	where
		for<'req> H: AuthEndpointHandler<'req, E> + Clone + Send + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>>,
		E::RequestHeaders: HasHeader<BearerToken>,
	{
		// All the layers needed
		let layer_builder = |state, handler, client_type| {
			ServiceBuilder::new()
				// .layer(todo!("Add rate limiter checker middleware here")),
				.layer(RequestParserLayer::with_state(state.clone()))
				.layer(AuthenticationLayer::new(client_type))
				// .layer(todo!("Add rate limiter value updater middleware here"))
				// .layer(todo!("Add audit logger middleware here"))
				.layer(AuthEndpointLayer::new(handler))
		};

		// Get the index for the current layer
		let index = API_CALL_REGISTRY_INDEX
			.write()
			.expect("API call registry index poisoned and cannot be opened for writing")
			.len();

		// Add the index and layer to the index and layer registry
		frontend::utils::API_CALL_REGISTRY_INDEX
			.write()
			.expect("API call registry index poisoned and cannot be opened for writing")
			.insert(
				format!(
					"{} {}",
					<E as ApiEndpoint>::METHOD,
					<E::RequestPath as TypedPath>::PATH
				),
				index,
			);

		// Setup the layers for the frontend
		frontend::utils::API_CALL_REGISTRY
			.write()
			.expect("API call registry poisoned and cannot be opened for writing")
			.push(layer_builder(
				state.clone(),
				handler.clone(),
				ClientType::WebDashboard,
			));

		// Setup the layers for the backend
		self.route(
			<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
			MethodRouter::<S>::new()
				.on(
					MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
					|| async { unreachable!() },
				)
				.layer(layer_builder(
					state.clone(),
					handler.clone(),
					ClientType::ApiToken,
				)),
		)
	}
}
