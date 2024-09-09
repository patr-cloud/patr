use std::{net::IpAddr, sync::RwLock};

use axum::{
	routing::{MethodFilter, MethodRouter},
	Router,
};
use axum_extra::routing::TypedPath;
use models::{
	utils::{HasHeader, NoAuthentication},
	ApiEndpoint,
};
use preprocess::Preprocessable;
use tower::{
	util::{BoxCloneService, BoxLayer},
	ServiceBuilder,
};

use crate::{
	prelude::*,
	utils::layers::{
		AuthenticationLayer,
		ClientType,
		DataStoreConnectionLayer,
		EndpointHandler,
		EndpointLayer,
		PreprocessLayer,
		RemoveIpAddrLayer,
		RequestParserLayer,
	},
};

pub trait RouterExt<S>
where
	S: Clone + Send + Sync + 'static,
{
	/// Mount an API endpoint directly along with the required request parser,
	/// and endpoint handler, using tower layers.
	#[track_caller]
	fn mount_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = NoAuthentication> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		R: RunnerExecutor + Clone + 'static;

	/// Mount an API endpoint directly along with the required request parser,
	/// Rate limiter, Audit logger and Auth middlewares, using tower layers.
	#[track_caller]
	fn mount_auth_endpoint<E, H, R>(self, handler: H, state: &AppState<R>) -> Self
	where
		for<'req> H: EndpointHandler<'req, E> + Clone + Send + Sync + 'static,
		E: ApiEndpoint<Authenticator = AppAuthentication<E>> + Sync,
		<E::RequestBody as Preprocessable>::Processed: Send,
		E::RequestHeaders: HasHeader<BearerToken>,
		R: RunnerExecutor + Clone + 'static;
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
		R: RunnerExecutor + Clone + 'static,
	{
		hosted_frontend::utils::API_CALL_REGISTRY
			.get_or_init(|| RwLock::new(Default::default()))
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
						// .layer(todo!("strip IP address and send"))
						.layer(RemoveIpAddrLayer::<E>::new())
						.layer(DataStoreConnectionLayer::<E, R>::with_state(state.clone()))
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

		// Setup the layers for the backend
		if <E as ApiEndpoint>::API_ALLOWED || cfg!(debug_assertions) {
			self.route(
				<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
				MethodRouter::<S>::new()
					.on(
						MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
						|| async {},
					)
					.layer(
						ServiceBuilder::new()
							.layer(RequestParserLayer::new())
							// .layer(todo!("Add rate limiter checker middleware here")),
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(EndpointLayer::new(handler)),
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
		R: RunnerExecutor + Clone + 'static,
	{
		hosted_frontend::utils::API_CALL_REGISTRY
			.get_or_init(|| RwLock::new(Default::default()))
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
						.layer(RemoveIpAddrLayer::<E>::new())
						.layer(DataStoreConnectionLayer::with_state(state.clone()))
						.layer(PreprocessLayer::new())
						.layer(AuthenticationLayer::new(ClientType::WebDashboard))
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

		// Setup the layers for the backend
		if <E as ApiEndpoint>::API_ALLOWED || cfg!(debug_assertions) {
			self.route(
				<<E as ApiEndpoint>::RequestPath as TypedPath>::PATH,
				MethodRouter::<S>::new()
					.on(
						MethodFilter::try_from(<E as ApiEndpoint>::METHOD).unwrap(),
						|| async {},
					)
					.layer(
						ServiceBuilder::new()
							.layer(RequestParserLayer::new())
							.layer(DataStoreConnectionLayer::with_state(state.clone()))
							.layer(PreprocessLayer::new())
							.layer(AuthenticationLayer::new(ClientType::ApiToken))
							.layer(EndpointLayer::new(handler)),
					),
			)
		} else {
			self
		}
	}
}
