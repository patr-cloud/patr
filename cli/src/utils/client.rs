use std::sync::OnceLock;

use models::{
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
	prelude::*,
	utils::{False, Headers},
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};

/// A reqwest client that can be used to make requests to the API
static REQUEST_CLIENT: OnceLock<Client> = OnceLock::new();

/// Make an API request to an endpoint
pub async fn make_request<E>(
	ApiRequest {
		path,
		query,
		headers,
		body,
	}: ApiRequest<E>,
) -> Result<ApiSuccessResponse<E>, ApiErrorResponse>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
	E::ResponseBody: DeserializeOwned + Serialize,
	E::RequestBody: DeserializeOwned + Serialize,
{
	let body = serde_json::to_value(&body)
		.map_err(|err| err.to_string())
		.map_err(|err| ApiErrorResponse {
			status_code: http::StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error(err.clone()),
				message: err,
			},
		})?;
	let query = serde_qs::to_string(&query)?;
	let builder = REQUEST_CLIENT
		.get_or_init(initialize_client)
		.request(
			E::METHOD,
			format!(
				"{}{}{}{}",
				super::constants::API_BASE_URL,
				path,
				if query.is_empty() { "" } else { "?" },
				query
			),
		)
		.headers({
			let mut headers = headers.to_header_map();
			headers.insert(
				reqwest::header::CONTENT_TYPE,
				reqwest::header::HeaderValue::from_static("application/json"),
			);
			headers
		});

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
				status_code: http::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			});
		}
	};

	let status_code = response.status();
	let Ok(headers) = E::ResponseHeaders::from_header_map(response.headers().clone()) else {
		return Err(ApiErrorResponse {
			status_code: http::StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error("invalid headers"),
				message: "invalid headers".to_string(),
			},
		});
	};

	match response.json::<ApiResponseBody<E::ResponseBody>>().await {
		Ok(ApiResponseBody::Success(ApiSuccessResponseBody {
			success: _,
			response: body,
		})) => Ok(ApiSuccessResponse {
			status_code: http::StatusCode::from_u16(status_code.as_u16())
				.expect("Status code is not valid"),
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code: http::StatusCode::from_u16(status_code.as_u16())
				.expect("Status code is not valid"),
			body: error,
		}),
		Err(error) => {
			error!("{}", error.to_string());
			Err(ApiErrorResponse {
				status_code: http::StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			})
		}
	}
}

/// Initialize a reqwest client that can be used across the application to make
/// requests
fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}
