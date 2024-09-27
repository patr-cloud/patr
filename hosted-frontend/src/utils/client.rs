use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::{any::Any, collections::HashMap, sync::RwLock};

use http::Method;
use leptos::ServerFnError;
use matchit::Router;
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType};
use preprocess::Preprocessable;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
/// The type used for the [`API_CALL_REGISTRY`] static. This is a map of all the
/// API calls that are registered to the backend. This is used internally and
/// should not be used by any other part of the code.
type ApiCallRegistryData = OnceLock<RwLock<HashMap<Method, Router<Box<dyn Any + Send + Sync>>>>>;

#[cfg(not(target_arch = "wasm32"))]
/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_request`] fn instead.
pub static API_CALL_REGISTRY: ApiCallRegistryData = OnceLock::new();

#[cfg(target_arch = "wasm32")]
/// The client used to make requests to the backend
static REQWEST_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Makes an API call to the backend. If you want to make an API request, just
/// call this function with the request and you'll get a response. All the
/// layering is automatically done. You don't need to do anything. The
/// registering of all APIs is done by the RouterExt trait in the backend
pub async fn make_request<E>(
	request: ApiRequest<E>,
) -> Result<AppResponse<E>, ServerFnError<ErrorType>>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	#[cfg(not(target_arch = "wasm32"))]
	{
		use std::net::{IpAddr, SocketAddr};

		use axum::extract::ConnectInfo;
		use axum_extra::routing::TypedPath;
		use tower::{
			service_fn,
			util::{BoxCloneService, BoxLayer},
			ServiceBuilder,
			ServiceExt,
		};

		let ConnectInfo(socket_addr) = leptos_axum::extract::<ConnectInfo<SocketAddr>>()
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
			.map_err(ServerFnError::WrappedServerError)
	}
	#[cfg(target_arch = "wasm32")]
	{
		use models::utils::Headers;

		let response = REQWEST_CLIENT
			.get_or_init(reqwest::Client::new)
			.request(E::METHOD, format!("/api/{}", request.path.to_string()))
			.headers(request.headers.to_header_map())
			.query(&request.query)
			.json(&request.body)
			.send()
			.await
			.map_err(|err| ServerFnError::Request(err.to_string()))?;

		let status_code = response.status();
		let headers = E::ResponseHeaders::from_header_map(response.headers())
			.map_err(|err| ServerFnError::Response(err.to_string()))?;
		let text = response
			.text()
			.await
			.map_err(|err| ServerFnError::Response(err.to_string()))?;
		let body =
			serde_json::from_str(&text).map_err(|err| ServerFnError::Response(err.to_string()))?;

		Ok(AppResponse {
			status_code,
			headers,
			body,
		})
	}
}
