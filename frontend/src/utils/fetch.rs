use std::str::FromStr;

use models::{
	api::ApiEndpoint,
	utils::{
		ApiRequest,
		ApiResponse,
		ApiResponseBody,
		ApiSuccessResponse,
		Headers,
	},
};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use super::constants;

/// Makes a request to the API. Requires an ApiRequest object for a specific
/// endpoint, and returns the response corresponding to that endpoint
/// TODO implement adding auth headers for protected endpoints and automatically
/// refreshing tokens
pub async fn make_request<T>(
	ApiRequest {
		path,
		query,
		headers,
		body,
	}: ApiRequest<T>,
) -> ApiResponse<T>
where
	T: ApiEndpoint,
	T::ResponseBody: DeserializeOwned + Serialize,
{
	let body = serde_json::to_value(&body).unwrap();
	let builder = reqwest::Client::new()
		.request(
			T::METHOD,
			Url::from_str(constants::API_BASE_URL)
				.unwrap()
				.join(path.to_string().as_str())
				.unwrap(),
		)
		.query(&query)
		.headers(headers.to_header_map());
	let response = if body.is_null() {
		builder
	} else {
		builder.json(&body)
	}
	.send()
	.await;

	let response = match response {
		Ok(response) => response,
		Err(error) => {
			return ApiResponse::internal_error(error.to_string());
		}
	};

	let status_code = response.status();
	let Some(headers) = T::ResponseHeaders::from_header_map(response.headers())
	else {
		return ApiResponse::internal_error("invalid headers");
	};

	match response.json::<ApiResponseBody<T::ResponseBody>>().await {
		Ok(ApiResponseBody::Success(ApiSuccessResponse {
			success: _,
			response: body,
		})) => ApiResponse::Success {
			status_code,
			headers,
			body,
		},
		Ok(ApiResponseBody::Error(error)) => ApiResponse::Error {
			status_code,
			body: error,
		},
		Err(error) => {
			log::error!("{}", error.to_string());
			return ApiResponse::internal_error(
				"Internal server occured parsing response",
			);
		}
	}
}
