use std::{any::Any, collections::BTreeMap, sync::OnceLock};

use axum_extra::routing::TypedPath;
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType};
use tower::{
	service_fn,
	util::{BoxCloneService, BoxLayer, ServiceExt},
	ServiceBuilder,
};

/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_api_call`] fn instead.
pub static API_CALL_REGISTRY: OnceLock<
	BTreeMap<
		String,
		BoxLayer<
			BoxCloneService<Box<dyn Any>, Box<dyn Any>, ErrorType>,
			Box<dyn Any>,
			Box<dyn Any>,
			ErrorType,
		>,
	>,
> = OnceLock::new();

pub(crate) async fn make_api_call<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
{
	ServiceBuilder::new()
		.layer(
			API_CALL_REGISTRY
				.get()
				.expect(concat!(
					"API call registry not initialized.",
					" Are you sure you are running the code with the backend?"
				))
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
				)
				.clone(),
		)
		.service(BoxCloneService::new(service_fn(|_| async move {
			unreachable!()
		})))
		.oneshot(Box::new(request))
		.await
		.map(|response| {
			*response
				.downcast::<AppResponse<E>>()
				.expect("invalid response")
		})
}
