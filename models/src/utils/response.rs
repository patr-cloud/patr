use axum::http::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{False, True};
use crate::{api::ApiEndpoint, ErrorType};

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ApiResponseBody<T> {
	Success(ApiSuccessResponse<T>),
	Error(ApiErrorResponse),
}

#[cfg(test)]
mod test {
	use super::ApiResponseBody;
	use crate::api::auth::LoginResponse;

	#[test]
	fn test() {
		let value: ApiResponseBody<LoginResponse> = serde_json::from_str(
			r#"
			{
				"success":true,
				"accessToken":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2OTEyNzA5MzksInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjkxNTMwMTM5LCJsb2dpbklkIjoiMDkyZTdkNTMwY2JmNGQ4NzllMTZjOWUxYTQxNGU5ZWYiLCJ1c2VyIjp7ImlkIjoiYjQ1NjBlOTUzMDkwNDE5NWEwOTk5YzZkMjZhYTljMjkiLCJ1c2VybmFtZSI6InNhbXlhay1nYW5nd2FsIiwiZmlyc3ROYW1lIjoiU2FteWFrIiwibGFzdE5hbWUiOiJHYW5nd2FsIiwiY3JlYXRlZCI6IjIwMjEtMDgtMTBUMTU6MDk6NDguMzAwWiJ9fQ.BdwtRgt7xNNhtLlhZAY0sSF5E-WgP0NSTNZFDhgQZ_Q",
				"refreshToken":"44036dc675824cdc962902bbd656c387",
				"loginId":"092e7d530cbf4d879e16c9e1a414e9ef"
			}
			"#
		).unwrap();
		println!("{:#?}", value);
	}
}
