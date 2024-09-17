use std::{str::FromStr, sync::OnceLock};

use futures::{Stream, StreamExt};
use http::{HeaderName, HeaderValue, StatusCode};
use models::{
	prelude::*,
	utils::{constants, False, Headers, WebSocketUpgrade},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::{
	client::IntoClientRequest,
	Error as TungsteniteError,
	Message,
};
use url::Url;

static REQUEST_CLIENT: OnceLock<Client> = OnceLock::new();

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
	E::ResponseBody: DeserializeOwned,
	E::RequestBody: Serialize,
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
			Url::from_str(crate::utils::constants::API_BASE_URL)
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
				status_code: StatusCode::INTERNAL_SERVER_ERROR,
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
					HeaderName::from_str(key.as_str()).unwrap(),
					HeaderValue::from_str(value.to_str().unwrap()).unwrap(),
				)
			})
			.collect(),
	) else {
		return Err(ApiErrorResponse {
			status_code: StatusCode::INTERNAL_SERVER_ERROR,
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
			status_code: StatusCode::from_u16(status_code.as_u16()).unwrap(),
			headers,
			body,
		}),
		Ok(ApiResponseBody::Error(error)) => Err(ApiErrorResponse {
			status_code: StatusCode::from_u16(status_code.as_u16()).unwrap(),
			body: error,
		}),
		Err(error) => {
			error!("{}", error.to_string());
			Err(ApiErrorResponse {
				status_code: StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(error.to_string()),
					message: error.to_string(),
				},
			})
		}
	}
}

pub async fn stream_request<E, ServerMsg, ClientMsg>(
	request: ApiRequest<E>,
) -> Result<impl Stream<Item = ServerMsg>, ApiErrorResponse>
where
	E: ApiEndpoint<RequestBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	ServerMsg: DeserializeOwned,
	ClientMsg: Serialize,
{
	let mut client_request = format!(
		"{}://{}{}",
		if constants::API_BASE_URL.starts_with("https") {
			"wss"
		} else {
			"ws"
		},
		constants::API_BASE_URL
			.trim_start_matches("https://")
			.trim_start_matches("http://"),
		request.path
	)
	.into_client_request()
	.map_err(|err| ApiErrorResponse {
		status_code: StatusCode::INTERNAL_SERVER_ERROR,
		body: ApiErrorResponseBody {
			success: False,
			error: ErrorType::server_error(err),
			message: err.to_string(),
		},
	})?;
	for (header, value) in request.headers.to_header_map().iter() {
		client_request
			.headers_mut()
			.insert(header.clone(), value.clone());
	}
	*client_request.method_mut() = E::METHOD;

	Ok(tokio_tungstenite::connect_async(client_request)
		.await
		.map_err(|err| match err {
			TungsteniteError::Http(err) => {
				let (parts, body) = err.into_parts();
				let body = body.unwrap_or_default();
				ApiErrorResponse {
					status_code: parts.status,
					body: serde_json::from_slice(&body).unwrap_or_else(|err| {
						error!("Failed to parse error body: {}", err);
						ApiErrorResponseBody {
							success: False,
							error: ErrorType::server_error(&err),
							message: err.to_string(),
						}
					}),
				}
			}
			err => ApiErrorResponse {
				status_code: StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(err),
					message: err.to_string(),
				},
			},
		})?
		.0
		.filter_map(|msg| async move {
			let msg = msg
				.inspect_err(|err| warn!("Error from websocket stream: {}", err))
				.ok()?;
			match msg {
				Message::Text(text) => serde_json::from_str(&text)
					.inspect_err(|err| warn!("Error parsing text as JSON: {}", err))
					.ok(),
				Message::Binary(bin) => serde_json::from_slice(&bin)
					.inspect_err(|err| {
						warn!(
							"Error parsing binary `{}` as JSON: {}",
							String::from_utf8_lossy(&bin),
							err
						)
					})
					.ok(),
				_ => None,
			}
		}))
}

fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}
