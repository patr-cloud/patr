use std::{str::FromStr, sync::OnceLock};

use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use url::Url;

use crate::{
	prelude::*,
	utils::{constants, False, Headers, WebSocketUpgrade},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
};

/// Contains a stream and a sink that gives data in a pre-defined format
mod websocket;

static REQUEST_CLIENT: OnceLock<Client> = OnceLock::new();

/// Makes a request to the API. Requires an ApiRequest object for a specific
/// endpoint, and returns the response corresponding to that endpoint
/// TODO implement automatically refreshing tokens
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
	E::ResponseBody: DeserializeOwned + Serialize,
	E::RequestBody: DeserializeOwned + Serialize,
{
	let body = serde_json::to_value(&body).unwrap();
	let builder = REQUEST_CLIENT
		.get_or_init(initialize_client)
		.request(
			E::METHOD,
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
	let Some(headers) = E::ResponseHeaders::from_header_map(response.headers()) else {
		return Err(ApiErrorResponse {
			status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
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
			status_code,
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code,
			body: error,
		}),
		Err(error) => {
			tracing::error!("{}", error.to_string());
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

/// Opens a websocket connection to the API. Requires an ApiRequest object for a
/// specific endpoint, and returns a websocket stream corresponding to that
/// endpoint TODO automatically refreshing tokens
pub async fn open_stream<E, ServerMsg, ClientMsg>(
	ApiRequest {
		path,
		query,
		headers,
		body: _,
	}: ApiRequest<E>,
) -> Result<websocket::WebSocketStream<ServerMsg, ClientMsg>, ApiErrorResponse>
where
	E: ApiEndpoint<RequestBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
{
	let mut request = Url::from_str(constants::API_BASE_URL)
		.unwrap()
		.join(path.to_string().as_str())
		.unwrap();

	let query = serde_urlencoded::to_string(&query).unwrap();
	request.set_query(
		if query.is_empty() {
			None
		} else {
			Some(query.as_str())
		},
	);

	let headers = headers.to_header_map();

	tokio_tungstenite::connect_async({
		let mut request = request.into_client_request().unwrap();

		request.headers_mut().extend(headers.into_iter());

		request
	})
	.await
	.map(|(stream, _)| websocket::WebSocketStream::<ServerMsg, ClientMsg>::new(stream))
	.map_err(|error| ApiErrorResponse {
		status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
		body: ApiErrorResponseBody {
			success: False,
			error: ErrorType::server_error(error.to_string()),
			message: error.to_string(),
		},
	})
}

fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}
