use std::fmt::Debug;

use axum::{
	Json,
	RequestExt as _,
	body::{Body, HttpBody as _},
	extract::{FromRequest, Path, Request},
	http::{Method, StatusCode},
	response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use models::utils::Headers;
use oci_spec::distribution::ErrorResponse;
use serde::{Serialize, de::DeserializeOwned};

use crate::prelude::*;

/// Trait that defines a registry endpoint. This trait is used to define
/// the types required for the request parsing of the registry endpoints
pub trait RegistryEndpoint {
	/// The HTTP method that should be used for this endpoint
	const METHOD: Method;
	/// Whether the endpoint requires authentication
	const AUTHENTICATED: bool;

	/// The URL path type for the endpoint
	type Path: TypedPath + Serialize + DeserializeOwned + Clone + Send + Sync + 'static;
	/// The request headers type for the endpoint
	type RequestHeaders: Headers + Debug + Send + Sync + 'static;
	/// The request query type for the endpoint
	type RequestQuery: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;

	/// The response headers type for the endpoint
	type ResponseHeaders: Headers + Debug + Send + Sync + 'static;
}

/// The request structure for registry endpoints
pub struct RegistryRequest<E>
where
	E: RegistryEndpoint,
{
	/// The parsed path of the request
	pub path: E::Path,
	/// The parsed headers of the request
	pub headers: E::RequestHeaders,
	/// The parsed query parameters of the request
	pub query: E::RequestQuery,
	/// The body of the request
	pub body: Body,
}

impl<E, S> FromRequest<S> for RegistryRequest<E>
where
	E: RegistryEndpoint,
	S: Send + Sync,
{
	type Rejection = Response;

	async fn from_request(mut req: Request, _: &S) -> Result<Self, Self::Rejection> {
		let Ok(Path(path)) = req.extract_parts().await.inspect_err(|err| {
			debug!("Failed to parse path `{}`: {}", req.uri().path(), err);
		}) else {
			return Err(Response::builder()
				.status(StatusCode::BAD_REQUEST)
				.body(Body::from("Invalid Path"))
				.unwrap());
		};
		let Ok(query) = serde_urlencoded::from_str(req.uri().query().unwrap_or_default())
			.inspect_err(|err| {
				debug!("Failed to parse query `{:?}`: {}", req.uri().query(), err);
			})
		else {
			return Err(Response::builder()
				.status(StatusCode::BAD_REQUEST)
				.body(Body::from("Invalid Query Parameters"))
				.unwrap());
		};

		let Ok(headers) = <E::RequestHeaders as Headers>::from_header_map(req.headers())
			.inspect_err(|err| {
				debug!("Failed to parse headers: {err}");
			})
		else {
			return Err(Response::builder()
				.status(StatusCode::BAD_REQUEST)
				.body(Body::from("Invalid Headers"))
				.unwrap());
		};
		let body = req.into_body();

		Ok(Self {
			path,
			headers,
			query,
			body,
		})
	}
}

/// The response structure for registry endpoints
pub struct RegistryResponse<E>
where
	E: RegistryEndpoint,
{
	/// The HTTP status code of the response
	pub status: StatusCode,
	/// The parsed headers of the response
	pub headers: E::ResponseHeaders,
	/// The body of the response
	pub body: Body,
}

impl<E> IntoResponse for RegistryResponse<E>
where
	E: RegistryEndpoint,
{
	fn into_response(self) -> Response {
		let mut response = Response::builder().status(self.status);

		for (key, value) in self.headers.to_header_map().iter() {
			response = response.header(key, value);
		}

		if let Some(size) = self.body.size_hint().exact() {
			response = response.header("Content-Length", size.to_string());
		}

		response
			.body(self.body)
			.expect("Failed to build registry response")
	}
}

/// The error type for registry endpoints
pub type RegistryError = Json<ErrorResponse>;
