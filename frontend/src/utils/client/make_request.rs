use std::sync::Arc;

use axum_extra::routing::TypedPath;
use http::Method;
use leptos::{
	server_fn::{
		client::browser::BrowserClient,
		codec::{Encoding, FromReq, GetUrl, IntoReq, PostUrl},
		middleware::Layer,
		request::{browser::BrowserRequest, ClientReq},
		ServerFn,
	},
	ServerFnError,
};
use models::prelude::*;
use preprocess::Preprocessable;
use serde::{de::DeserializeOwned, Serialize};

/// A struct that holds the request to be made to the backend. This is used
/// for the server fn to make the request to the backend.
struct MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	request: ApiRequest<E>,
}

impl<E> ServerFn for MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	type Client = BrowserClient;
	type Error = ErrorType;
	type InputEncoding = ApiEncoding<E>;
	type Output = AppResponse<E>;
	type OutputEncoding = ApiEncoding<E>;
	#[cfg(not(target_arch = "wasm32"))]
	type ServerRequest = http::Request<axum::body::Body>;
	#[cfg(target_arch = "wasm32")]
	type ServerRequest = leptos::server_fn::request::BrowserMockReq;
	#[cfg(not(target_arch = "wasm32"))]
	type ServerResponse = http::Response<axum::body::Body>;
	#[cfg(target_arch = "wasm32")]
	type ServerResponse = leptos::server_fn::response::BrowserMockRes;

	const PATH: &'static str = <E::RequestPath as TypedPath>::PATH;

	#[cfg(not(target_arch = "wasm32"))]
	async fn run_body(self) -> Result<Self::Output, ServerFnError<Self::Error>> {
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
		let layer = super::API_CALL_REGISTRY
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
			.map_err(ServerFnError::WrappedServerError)
	}

	#[cfg(target_arch = "wasm32")]
	async fn run_body(self) -> Result<Self::Output, ServerFnError<Self::Error>> {
		unreachable!()
	}

	fn middlewares() -> Vec<Arc<dyn Layer<Self::ServerRequest, Self::ServerResponse>>> {
		vec![]
	}
}

impl<E> IntoReq<ApiEncoding<E>, BrowserRequest, ErrorType> for MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	fn into_req(
		self,
		path: &str,
		accepts: &str,
	) -> Result<BrowserRequest, ServerFnError<ErrorType>> {
		if E::METHOD == Method::GET {
			BrowserRequest::try_new_get(
				path,
				GetUrl::CONTENT_TYPE,
				accepts,
				serde_urlencoded::to_string(self.request.query)
					.unwrap()
					.as_str(),
			)
		} else {
			BrowserRequest::try_new_post(
				path,
				PostUrl::CONTENT_TYPE,
				accepts,
				serde_json::to_string(&self.request.body).unwrap(),
			)
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<E> FromReq<ApiEncoding<E>, http::Request<axum::body::Body>, ErrorType> for MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	async fn from_req(
		_req: http::Request<axum::body::Body>,
	) -> Result<Self, ServerFnError<ErrorType>> {
		todo!()
	}
}

#[cfg(target_arch = "wasm32")]
impl<E> FromReq<ApiEncoding<E>, leptos::server_fn::request::BrowserMockReq, ErrorType>
	for MakeRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	async fn from_req(
		req: leptos::server_fn::request::BrowserMockReq,
	) -> Result<Self, ServerFnError<ErrorType>> {
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
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	#[cfg(not(target_arch = "wasm32"))]
	{
		MakeRequest { request }.run_body().await
	}
	#[cfg(target_arch = "wasm32")]
	{
		MakeRequest { request }.run_on_client().await
	}
}

/// Register an API call to the backend. This will register the API call to the
/// backend so that it can be used by the frontend. This is used internally and
/// should not be used by any other part of the code.
#[cfg(not(target_arch = "wasm32"))]
pub fn register_api_call<E>()
where
	E: ApiEndpoint,
	E::RequestBody: Serialize + DeserializeOwned,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	leptos::server_fn::axum::register_explicit::<MakeRequest<E>>();
}
