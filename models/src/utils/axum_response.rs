use axum::{
	response::{IntoResponse, Response},
	Json,
};
use serde::{de::DeserializeOwned, Serialize};

use super::True;
use crate::ApiSuccessResponseBody;

/// This trait is implemented for all types that can be used as a response to an
/// API endpoint. This is used to convert the type to a [`Response`] that can be
/// returned from an endpoint.
///
/// This trait is automatically implemented for any type that implements
/// [`Serialize`] and [`DeserializeOwned`], and will return a JSON response.
///
/// This trait is also implemented for [`GenericResponse`], which can be used to
/// return a custom [`Response`] from an endpoint (mostly used for WebSocket
/// responses).
pub trait IntoAxumResponse {
	/// Convert the type to a [`Response`] that can be used with [`axum`].
	fn into_axum_response(self) -> Response;
}

impl<T> IntoAxumResponse for T
where
	T: Serialize + DeserializeOwned,
{
	fn into_axum_response(self) -> Response {
		Json(ApiSuccessResponseBody {
			success: True,
			response: self,
		})
		.into_response()
	}
}

/// A type that can be used to return a custom [`Response`] from an endpoint.
/// This is mostly used for WebSocket responses, among other streaming responses
/// that cannot be represented by a JSON response.
#[derive(Debug, Default)]
pub struct GenericResponse(pub Response);

impl IntoAxumResponse for GenericResponse {
	fn into_axum_response(self) -> Response {
		self.0
	}
}