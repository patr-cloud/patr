use std::future::Future;

use axum::{body::Body, http::Request, Json, RequestExt};
use serde::{de::DeserializeOwned, Serialize};

use crate::ErrorType;

/// A trait that defines a type that can be parsed from an axum request. This is
/// used to parse the request body into a struct. This is implemented for any
/// type that implements [`serde::Serialize`] and [`serde::Deserialize`] as
/// JSON, and for websocket requests
pub trait FromAxumRequest
where
	Self: Sized,
{
	/// Parses the given request and extracts the body
	fn from_axum_request(
		request: Request<Body>,
	) -> impl Future<Output = Result<Self, ErrorType>> + Send;
}

impl<T> FromAxumRequest for T
where
	T: Serialize + DeserializeOwned + 'static,
{
	#[tracing::instrument(skip(request))]
	fn from_axum_request(request: Request<Body>) -> impl Future<Output = Result<Self, ErrorType>> {
		async move {
			request
				.extract()
				.await
				.map_err(|err| {
					tracing::debug!("Failed to parse body: {}", err);
					ErrorType::WrongParameters
				})
				.map(|Json(body)| body)
		}
	}
}
