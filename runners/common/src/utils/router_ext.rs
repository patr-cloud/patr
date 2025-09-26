use std::{collections::HashMap, net::IpAddr, sync::RwLock};

use axum::Router;
use axum_extra::routing::TypedPath;
use models::{
	ApiEndpoint,
	utils::{HasHeader, NoAuthentication},
};
use preprocess::Preprocessable;
use serde::{Serialize, de::DeserializeOwned};
use tower::{
	ServiceBuilder,
	util::{BoxCloneService, BoxLayer},
};

use crate::{
	prelude::*,
	utils::layers::{
		AuthenticationLayer,
		DataStoreConnectionLayer,
		EndpointHandler,
		EndpointLayer,
		PreprocessLayer,
		RemoveIpAddrLayer,
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
	/// Mount a JSON API endpoint directly along with the required request
	/// parser, and endpoint handler, using tower layers.
	#[track_caller]
	fn mount_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestBody: Serialize + DeserializeOwned,
		E::ResponseBody: Serialize + DeserializeOwned,
		R: RunnerExecutor + Send + 'static;

	/// Mount an API endpoint directly along with the required request parser,
	/// and endpoint handler, using tower layers.
	#[track_caller]
	#[must_use]
	fn mount_endpoint_stream<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		R: RunnerExecutor + Send + 'static;

	/// Mount a JSON API endpoint directly along with the required request
	/// parser, Rate limiter, Audit logger and Auth middlewares, using tower
	/// layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		E::RequestBody: Serialize + DeserializeOwned,
		E::ResponseBody: Serialize + DeserializeOwned,
		R: RunnerExecutor + Send + 'static;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	#[must_use]
	fn mount_auth_endpoint_stream<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
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
	fn mount_endpoint_stream<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		R: RunnerExecutor + Send + 'static,
	{
		frontend::utils::API_CALL_REGISTRY
			.get_or_init(|| RwLock::new(HashMap::default()))
			.write()
			.expect("API call registry poisoned")
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
						.layer(RemoveIpAddrLayer::new())
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						.layer(PreprocessLayer::new())
						.layer(EndpointLayer::new(handler.clone())),
				)),
			)
			.unwrap_or_else(|_| {
				panic!(
					"API endpoint `{} {}` already registered",
					E::METHOD,
					<E::RequestPath as TypedPath>::PATH
				);
			});

		self
	}

	#[instrument(skip_all)]
	fn mount_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestBody: Serialize + DeserializeOwned,
		E::ResponseBody: Serialize + DeserializeOwned,
		R: RunnerExecutor + Send + 'static,
	{
		frontend::utils::register_api_call::<E>();

		Self::mount_endpoint_stream(self, handler, state)
	}

	#[instrument(skip_all)]
	fn mount_auth_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		E::RequestBody: Serialize + DeserializeOwned,
		E::ResponseBody: Serialize + DeserializeOwned,
		R: RunnerExecutor + Send + 'static,
	{
		frontend::utils::register_api_call::<E>();

		Self::mount_auth_endpoint_stream(self, handler, state)
	}

	#[instrument(skip_all)]
	fn mount_auth_endpoint_stream<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		R: RunnerExecutor + Send + 'static,
	{
		frontend::utils::API_CALL_REGISTRY
			.get_or_init(|| RwLock::new(HashMap::default()))
			.write()
			.expect("API call registry poisoned")
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
						.layer(RemoveIpAddrLayer::new())
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						.layer(PreprocessLayer::new())
						.layer(AuthenticationLayer::new())
						.layer(EndpointLayer::new(handler.clone())),
				)),
			)
			.unwrap_or_else(|_| {
				panic!(
					"API endpoint `{} {}` already registered",
					E::METHOD,
					<E::RequestPath as TypedPath>::PATH
				);
			});

		self
	}
}
