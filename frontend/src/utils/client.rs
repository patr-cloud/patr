use std::{
	any::Any,
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	sync::{OnceLock, RwLock},
};

use axum::extract::ConnectInfo;
use http::Method;
use axum_extra::routing::TypedPath;
use matchit::Router;
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType};
use preprocess::Preprocessable;
use tower::{
	service_fn,
	util::{BoxCloneService, BoxLayer},
	ServiceBuilder,
	ServiceExt,
};

/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_api_call`] fn instead.
pub static API_CALL_REGISTRY: RwLock<
	OnceLock<HashMap<Method, Router<Box<dyn Any + Send + Sync>>>>,
> = RwLock::new(OnceLock::new());

/// Makes an API call to the backend. If you want to make an API request, just
/// call this function with the request and you'll get a response. All the
/// layering is automatically done. You don't need to do anything. The
/// registering of all APIs is done by the RouterExt trait in the backend
pub(crate) async fn make_api_call<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	let ConnectInfo(socket_addr) =
		leptos_axum::extract::<ConnectInfo<SocketAddr>>()
			.await
			.map_err(ErrorType::server_error)?;
	let layer = API_CALL_REGISTRY
		.read()
		.expect("API call registry poisoned")
		.get()
		.expect("API call registry not initialized")
		.get(&E::METHOD)
		.expect(&format!("API call registry does not contain {}", E::METHOD))
		.at(<E::RequestPath as TypedPath>::PATH)
		.expect(&format!(
			"could not find route at path `{}`",
			<E::RequestPath as TypedPath>::PATH
		))
		.value
		.downcast_ref::<BoxLayer<
			BoxCloneService<(ApiRequest<E>, IpAddr), AppResponse<E>, ErrorType>,
			(ApiRequest<E>, IpAddr),
			AppResponse<E>,
			ErrorType,
		>>()
		.expect("unable to downcast layer")
		.to_owned();
	ServiceBuilder::new()
		.layer(layer)
		.service(BoxCloneService::new(service_fn(|_| async move {
			unreachable!()
		})))
		.oneshot((request, socket_addr.ip()))
		.await
}
