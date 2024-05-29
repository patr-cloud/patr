use std::{str::FromStr, sync::OnceLock};

use models::{
	prelude::*,
	utils::{False, Headers},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

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
	let builder = REQUEST_CLIENT
		.get_or_init(initialize_client)
		.request(
			reqwest::Method::from_str(E::METHOD.as_ref()).unwrap(),
			Url::from_str(super::constants::API_BASE_URL)
				.unwrap()
				.join(path.to_string().as_str())
				.unwrap(),
		)
		.query(&query)
		.headers(
			headers
				.to_header_map()
				.into_iter()
				.filter_map(|(key, value)| {
					Some((
						reqwest::header::HeaderName::from_str(key?.as_str()).unwrap(),
						reqwest::header::HeaderValue::from_str(value.to_str().unwrap()).unwrap(),
					))
				})
				.chain([(
					reqwest::header::CONTENT_TYPE,
					reqwest::header::HeaderValue::from_static("application/json"),
				)])
				.collect(),
		);

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
	let Ok(headers) = E::ResponseHeaders::from_header_map(
		&response
			.headers()
			.into_iter()
			.map(|(key, value)| {
				(
					http::HeaderName::from_str(key.as_str()).unwrap(),
					http::header::HeaderValue::from_str(value.to_str().unwrap()).unwrap(),
				)
			})
			.collect(),
	) else {
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
			status_code: http::StatusCode::from_u16(status_code.as_u16()).unwrap(),
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code: http::StatusCode::from_u16(status_code.as_u16()).unwrap(),
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
