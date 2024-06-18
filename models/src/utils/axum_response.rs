use axum::{
	response::{IntoResponse, Response},
	Json,
};
use serde::{de::DeserializeOwned, Serialize};

use super::{RequiresRequestHeaders, RequiresResponseHeaders, True};
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

	/// Check if the type is the same as the type parameter.
	fn is<T>(&self) -> bool
	where
		T: 'static,
		Self: 'static,
	{
		std::any::TypeId::of::<T>() == std::any::TypeId::of::<Self>()
	}
}

impl<T> IntoAxumResponse for T
where
	T: Serialize + DeserializeOwned,
{
	fn into_axum_response(self) -> Response {
		match serde_json::to_value(self).unwrap() {
			serde_json::Value::Null => Json(ApiSuccessResponseBody {
				success: True,
				response: (),
			})
			.into_response(),
			other => Json(ApiSuccessResponseBody {
				success: True,
				response: other,
			})
			.into_response(),
		}
	}
}

/// A type that can be used to return a custom [`Response`] from an endpoint.
/// This is mostly used for WebSocket responses, among other streaming responses
/// that cannot be represented by a JSON response.
#[derive(Debug)]
pub struct GenericResponse(pub Response);

impl IntoAxumResponse for GenericResponse {
	fn into_axum_response(self) -> Response {
		self.0
	}
}

impl RequiresRequestHeaders for GenericResponse {
	type RequiredRequestHeaders = ();
}

impl RequiresResponseHeaders for GenericResponse {
	type RequiredResponseHeaders = ();
}
