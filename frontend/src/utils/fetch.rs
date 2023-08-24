use std::str::FromStr;

use models::{
	utils::{
		ApiErrorResponse,
		ApiErrorResponseBody,
		ApiRequest,
		ApiResponseBody,
		ApiSuccessResponse,
		ApiSuccessResponseBody,
		False,
		Headers,
	},
	ApiEndpoint,
	ErrorType,
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
) -> Result<ApiSuccessResponse<T>, ApiErrorResponse>
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
			return Err(ApiErrorResponse {
				status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			});
		}
	};

	let status_code = response.status();
	let Some(headers) = T::ResponseHeaders::from_header_map(response.headers())
	else {
		return Err(ApiErrorResponse {
			status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error("invalid headers"),
				message: "invalid headers".to_string(),
			},
		});
	};

	match response.json::<ApiResponseBody<T::ResponseBody>>().await {
		Ok(ApiResponseBody::Success(ApiSuccessResponseBody {
			success: _,
			response: body,
		})) => Ok(ApiSuccessResponse {
			status_code,
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code,
			body: error,
		}),
		Err(error) => {
			log::error!("{}", error.to_string());
			Err(ApiErrorResponse {
				status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			})
		}
	}
}
