use std::{
	any::Any,
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	sync::{OnceLock, RwLock},
};

use axum::extract::ConnectInfo;
use axum_extra::routing::TypedPath;
use http::Method;
use matchit::Router;
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType, utils::Headers};
use preprocess::Preprocessable;
use serde::{Serialize, de::DeserializeOwned};
use tower::{
	ServiceBuilder,
	ServiceExt,
	service_fn,
	util::{BoxCloneService, BoxLayer},
};

use crate::prelude::*;

/// The type used for the [`API_CALL_REGISTRY`] static. This is a map of all the
/// API calls that are registered to the backend. This is used internally and
/// should not be used by any other part of the code.
type ApiCallRegistryData = OnceLock<RwLock<HashMap<Method, Router<Box<dyn Any + Send + Sync>>>>>;

/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_api_call`] fn instead.
pub static API_CALL_REGISTRY: ApiCallRegistryData = OnceLock::new();

/// Makes an API call to the backend. If you want to make an API request, just
/// call this function with the request and you'll get a response. All the
/// layering is automatically done. You don't need to do anything. The
/// registering of all APIs is done by the RouterExt trait in the backend
pub(crate) async fn make_api_call<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned + Clone,
	E::ResponseBody: Serialize + DeserializeOwned + Clone,
{
	// TODO fix this after we figure out how to handle server related stuff
	if cfg!(debug_assertions) {
		let api_request = reqwest::Client::new()
			.request(
				E::METHOD,
				format!("http://localhost:3000{}", request.path.to_string()),
			)
			.query(&request.query)
			.headers(request.headers.to_header_map());

		let response = if serde_json::to_value(request.body.clone())
			.unwrap()
			.is_null()
		{
			api_request.send()
		} else {
			api_request.json(&request.body).send()
		}
		.await
		.map_err(|err| ErrorType::server_error(format!("Failed to make API call: {}", err)))?;

		let response = AppResponse::<E> {
			status_code: response.status(),
			headers: E::ResponseHeaders::from_header_map(&response.headers())
				.map_err(ErrorType::server_error)?,
			body: response.json::<E::ResponseBody>().await.map_err(|err| {
				ErrorType::server_error(format!("Failed to parse response body: {}", err))
			})?,
		};

		return Ok(response);
	}

	let ConnectInfo(socket_addr) = extract::<ConnectInfo<SocketAddr>, _>()
		.await
		.map_err(ErrorType::server_error)?;
	let layer = API_CALL_REGISTRY
		.get()
		.expect("API call registry not initialized")
		.read()
		.expect("API call registry poisoned")
		.get(&E::METHOD)
		.unwrap_or_else(|| panic!("API call registry does not contain {}", E::METHOD))
		.at(<E::RequestPath as TypedPath>::PATH)
		.unwrap_or_else(|_| {
			panic!(
				"could not find route at path `{}`",
				<E::RequestPath as TypedPath>::PATH
			)
		})
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
