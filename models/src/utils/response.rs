use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use super::{False, Headers, IntoAxumResponse, True};
use crate::{ApiEndpoint, ErrorType};

#[derive(Debug)]
pub struct ApiSuccessResponse<E>
where
	E: ApiEndpoint,
{
	pub status_code: StatusCode,
	pub headers: E::ResponseHeaders,
	pub body: E::ResponseBody,
}

impl<E> IntoResponse for ApiSuccessResponse<E>
where
	E: ApiEndpoint,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSuccessResponseBody<T> {
	pub success: True,
	#[serde(flatten)]
	pub response: T,
}

#[derive(Debug, Clone)]
pub struct ApiErrorResponse {
	pub status_code: StatusCode,
	pub body: ApiErrorResponseBody,
}

impl ApiErrorResponse {
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

	pub fn error_with_message(
		error: ErrorType,
		message: impl Into<String>,
	) -> Self {
		Self {
			status_code: error.default_status_code(),
			body: ApiErrorResponseBody {
				success: False,
				error,
				message: message.into(),
			},
		}
	}

	pub fn internal_error(message: impl Into<String>) -> Self {
		Self::error(ErrorType::InternalServerError(anyhow::Error::msg(
			message.into(),
		)))
	}
}

impl IntoResponse for ApiErrorResponse {
	fn into_response(self) -> axum::response::Response {
		(self.status_code, Json(self.body)).into_response()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponseBody {
	pub success: False,
	pub error: ErrorType,
	pub message: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ApiResponseBody<T> {
	Success(ApiSuccessResponseBody<T>),
	Error(ApiErrorResponseBody),
}
