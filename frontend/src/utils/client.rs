use std::{any::Any, collections::BTreeMap, sync::RwLock};

use axum_extra::routing::TypedPath;
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType};
use tower::{
	service_fn,
	steer::{Picker, Steer},
	util::{BoxCloneService, BoxLayer, ServiceExt},
	ServiceBuilder,
};

/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_api_call`] fn instead.
pub static API_CALL_REGISTRY_INDEX: RwLock<BTreeMap<String, usize>> = RwLock::new(BTreeMap::new());

/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_api_call`] fn instead.
pub static API_CALL_REGISTRY: RwLock<Vec<Box<dyn Any + Send + Sync>>> = RwLock::new(Vec::new());

/// Makes an API call to the backend. If you want to make an API request, just
/// call this function with the request and you'll get a response. All the
/// layering is automatically done. You don't need to do anything. The
/// registering of all APIs is done by the RouterExt trait in the backend
pub(crate) async fn make_api_call<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
{
	let index = *API_CALL_REGISTRY_INDEX
		.read()
		.expect("API call registry index poisoned")
		.get(&format!(
			"{} {}",
			E::METHOD,
			<E::RequestPath as TypedPath>::PATH
		))
		.expect(
			format!(
				"API call registry for the route `{} {}` not registered.",
				E::METHOD,
				<E::RequestPath as TypedPath>::PATH
			)
			.as_str(),
		);

	Steer::new(
		API_CALL_REGISTRY
			.read()
			.expect("API call registry poisoned")
			.iter()
			.map(|service| {
				service
					.downcast_ref::<BoxCloneService<ApiRequest<E>, AppResponse<E>, ErrorType>>()
					.expect("invalid API call")
			}),
		move |_, _| index,
	)
	.oneshot(request)
	.await
	.map(|response| {
		*response
			.downcast::<AppResponse<E>>()
			.expect("invalid response")
	})
}
