use std::fmt::Display;

use axum::{http::StatusCode, response::IntoResponse, Json};
use preprocess::Preprocessable;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{
	prelude::*,
	utils::{False, Headers, IntoAxumResponse, True},
};

/// A response object that is passed through the tower layers and services
#[derive(Debug, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
pub struct AppResponse<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The status code of the response
	pub status_code: StatusCode,
	/// The headers of the response
	pub headers: E::ResponseHeaders,
	/// The body of the response
	pub body: E::ResponseBody,
}

impl<E> AppResponse<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Convert the response into a Result
	pub fn into_result(self) -> Result<Self, ErrorType> {
		Ok(self)
	}
}

/// This struct represents a successful response from the API. It contains the
/// status code, headers and body.
#[derive(Debug)]
pub struct ApiSuccessResponse<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The status code of the success response. Ideally in the 2xx range.
	pub status_code: StatusCode,
	/// The headers of the success response.
	pub headers: E::ResponseHeaders,
	/// The body of the success response. This is the actual data that will be
	/// sent to the client. Can be either JSON or Websockets.
	pub body: E::ResponseBody,
}

impl<E> IntoResponse for ApiSuccessResponse<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	fn into_response(self) -> axum::response::Response {
		(
			self.status_code,
			self.headers.to_header_map(),
			self.body.into_axum_response(),
		)
			.into_response()
	}
}

/// This struct represents the JSON body of successful response from the API.
/// This is mostly used internally and would ideally not need to be constructed
/// manually.
///
/// Use [`ApiSuccessResponse`] to create a success response.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSuccessResponseBody<T> {
	/// Whether the request was successful or not. This is always true.
	pub success: True,
	/// The JSON body of the response. This is flattened so that the fields of
	/// the body are at the top level.
	#[serde(flatten)]
	pub response: T,
}

impl<T> ApiSuccessResponseBody<T> {
	/// Creates a new [`ApiSuccessResponseBody`] with the given response.
	pub fn new(response: T) -> Self {
		Self {
			success: True,
			response,
		}
	}
}

impl ApiSuccessResponseBody<()> {
	/// Creates a new [`ApiSuccessResponseBody`] with an empty response.
	pub fn empty() -> Self {
		Self {
			success: True,
			response: (),
		}
	}
}

/// This struct represents an error response from the API. It contains the
/// status code and the body of the response.
#[derive(Debug, Clone)]
pub struct ApiErrorResponse {
	/// The status code of the error response. Ideally in the 4xx or 5xx range.
	pub status_code: StatusCode,
	/// The body of the error response. This is a JSON object that contains the
	/// error message.
	pub body: ApiErrorResponseBody,
}

impl ApiErrorResponse {
	/// Creates a new [`ApiErrorResponse`] with the given [`ErrorType`], using
	/// the default status code.
	pub fn error(error: ErrorType) -> Self {
		Self {
			status_code: error.default_status_code(),
			body: ApiErrorResponseBody {
				success: False,
				message: error.message().into(),
				error,
			},
		}
	}

	/// Creates a new [`ApiErrorResponse`] with the given [`ErrorType`] and the
	/// given message, using the default status code.
	pub fn error_with_message(error: ErrorType, message: impl Display) -> Self {
		Self {
			status_code: error.default_status_code(),
			body: ApiErrorResponseBody {
				success: False,
				error,
				message: message.to_string(),
			},
		}
	}

	/// Creates a new [`ApiErrorResponse`] with the given message as an internal
	/// server error.
	pub fn internal_error(message: impl Display) -> Self {
		error!("Internal server error: `{}`", message);
		Self::error(ErrorType::InternalServerError)
	}
}

impl IntoResponse for ApiErrorResponse {
	fn into_response(self) -> axum::response::Response {
		(self.status_code, Json(self.body)).into_response()
	}
}

impl<E> From<E> for ApiErrorResponse
where
	E: std::error::Error,
{
	fn from(err: E) -> Self {
		ApiErrorResponse::internal_error(err.to_string())
	}
}

/// This struct represents the JSON body of an error response from the API.
/// This is mostly used internally and would ideally not need to be constructed
/// manually.
///
/// Use [`ApiErrorResponse`] to create an error response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponseBody {
	/// Whether the request was successful or not. This is always false.
	pub success: False,
	/// The error type of the response.
	pub error: ErrorType,
	/// A user-friendly message describing the error.
	pub message: String,
}

/// This struct represents the JSON body of a response from the API.
///
/// It can be either a success or an error response. This is mostly used
/// internally and would ideally not need to be constructed manually. This is
/// used to parse the response from the API and determine whether it was
/// successful or not.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ApiResponseBody<T> {
	/// Success response, with the given body.
	Success(ApiSuccessResponseBody<T>),
	/// Error response
	Error(ApiErrorResponseBody),
}
