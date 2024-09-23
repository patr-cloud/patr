use std::{
	any::Any,
	collections::HashMap,
	sync::{Arc, OnceLock, RwLock},
};

use axum_extra::routing::TypedPath;
use http::Method;
use leptos::server_fn::{
	client::browser::BrowserClient,
	codec::Json,
	const_format::concatcp,
	middleware::Layer,
	ServerFn,
};
use models::{ApiEndpoint, ApiRequest, AppResponse, ErrorType};
use preprocess::Preprocessable;

/// The type used for the [`API_CALL_REGISTRY`] static. This is a map of all the
/// API calls that are registered to the backend. This is used internally and
/// should not be used by any other part of the code.
type ApiCallRegistryData = OnceLock<RwLock<HashMap<Method, Router<Box<dyn Any + Send + Sync>>>>>;

#[cfg(not(target_arch = "wasm32"))]
/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_request`] fn instead.
pub static API_CALL_REGISTRY: ApiCallRegistryData = OnceLock::new();

struct MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The request to be made to the backend
	request: ApiRequest<E>,
}

impl<E> ServerFn for MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Client = BrowserClient;
	type Error = ErrorType;
	type InputEncoding = Json;
	type Output = AppResponse<E>;
	type OutputEncoding = Json;
	#[cfg(not(target_arch = "wasm32"))]
	type ServerRequest = http::Request<axum::body::Body>;
	#[cfg(target_arch = "wasm32")]
	type ServerRequest = leptos::server_fn::request::BrowserMockReq;
	#[cfg(not(target_arch = "wasm32"))]
	type ServerResponse = http::Response<axum::body::Body>;
	#[cfg(target_arch = "wasm32")]
	type ServerResponse = leptos::server_fn::response::BrowserMockRes;

	const PATH: &'static str = get_endpoint_path::<E>();

	fn middlewares() -> Vec<Arc<dyn Layer<Self::ServerRequest, Self::ServerResponse>>> {
		// TODO change the middlewares based on the endpoint
		vec![]
	}

	#[cfg(not(target_arch = "wasm32"))]
	async fn run_body(self) -> Result<Self::Output, ServerFnError<Self::Error>> {
		use std::{
			marker::PhantomData,
			net::{IpAddr, SocketAddr},
		};

		use axum::extract::ConnectInfo;
		use leptos::ServerFnError;
		use matchit::Router;
		use serde::Serialize;
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
			.oneshot((self.request, socket_addr.ip()))
			.await
	}

	#[cfg(target_arch = "wasm32")]
	async fn run_body(self) -> Result<Self::Output, ServerFnError<Self::Error>> {
		unreachable!()
	}
}

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
{
	#[cfg(not(target_arch = "wasm32"))]
	let response = MakeRequest::<E> { request }.run_body().await;
	#[cfg(target_arch = "wasm32")]
	let response = MakeRequest::<E> { request }.run_on_client().await;
	response
}

#[cfg(not(target_arch = "wasm32"))]
/// Used internally for registering API calls to the backend. DO NOT USE THIS ON
/// YOUR OWN. Use the [`make_request`] fn instead.
pub fn register_request<E>()
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	leptos::server_fn::axum::register_explicit::<MakeRequest<E>>();
}

const fn get_endpoint_path<E>() -> &'static str
where
	E: ApiEndpoint,
{
	match E::METHOD {
		Method::GET => E::RequestPath::PATH,
		Method::POST => concatcp!(E::RequestPath::PATH, "/create"),
		Method::PUT | Method::PATCH => concatcp!(E::RequestPath::PATH, "/update"),
		Method::DELETE => concatcp!(E::RequestPath::PATH, "/delete"),
		_ => panic!("Unsupported method: {}", E::METHOD),
	}
}
