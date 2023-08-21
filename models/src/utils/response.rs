use axum::http::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{False, True};
use crate::{ApiEndpoint, ErrorType};

#[derive(Debug)]
pub enum ApiResponse<E>
where
	E: ApiEndpoint,
	E::ResponseBody: Serialize + DeserializeOwned,
{
	Success {
		status_code: StatusCode,
		headers: E::ResponseHeaders,
		body: E::ResponseBody,
	},
	Error {
		status_code: StatusCode,
		body: ApiErrorResponse,
	},
}

impl<T> ApiResponse<T>
where
	T: ApiEndpoint,
	T::ResponseBody: Serialize + DeserializeOwned,
{
	pub fn success(
		status_code: StatusCode,
		headers: T::ResponseHeaders,
		response: T::ResponseBody,
	) -> Self {
		Self::Success {
			status_code,
			headers,
			body: response,
		}
	}

	pub fn error(error: ErrorType) -> Self {
		Self::Error {
			status_code: error.default_status_code(),
			body: ApiErrorResponse {
				success: False,
				message: error.message().into(),
				error,
			},
		}
	}

	pub fn error_with_message(error: ErrorType, message: String) -> Self {
		Self::Error {
			status_code: error.default_status_code(),
			body: ApiErrorResponse {
				success: False,
				error,
				message,
			},
		}
	}

	pub fn internal_error(message: impl Into<String>) -> Self {
		let message = message.into();
		Self::error_with_message(
			ErrorType::InternalServerError(anyhow::Error::msg(message.clone())),
			message,
		)
	}
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiSuccessResponse<T> {
	pub success: True,
	#[serde(flatten)]
	pub response: T,
}

impl<T> ApiSuccessResponse<T> {
	pub fn into_response(self) -> T {
		self.response
	}
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
	pub success: False,
	pub error: ErrorType,
	pub message: String,
}

impl ApiErrorResponse {
	pub fn new(error: ErrorType) -> Self {
		Self {
			success: False,
			message: error.message().into(),
			error,
		}
	}

	pub fn new_with_message(
		error: ErrorType,
		message: impl Into<String>,
	) -> Self {
		Self {
			success: False,
			error,
			message: message.into(),
		}
	}

	pub fn internal_error(message: impl Into<String>) -> Self {
		let message = message.into();
		Self::new_with_message(
			ErrorType::InternalServerError(anyhow::Error::msg(message.clone())),
			message,
		)
	}
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ApiResponseBody<T> {
	Success(ApiSuccessResponse<T>),
	Error(ApiErrorResponse),
}
