use std::{net::IpAddr, sync::RwLock};

use axum::Router;
use axum_extra::routing::TypedPath;
use models::{utils::NoAuthentication, ApiEndpoint};
use tower::{
	util::{BoxCloneService, BoxLayer},
	ServiceBuilder,
};

use crate::prelude::*;
// use crate::utils::layers::EndpointHandler;

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
		R: RunnerExecutor;
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
		R: RunnerExecutor,
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
						.layer(DataStoreConnectionLayer::<E>::with_state(state.clone()))
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
	}
}
