use std::{fmt::Debug, io::Error as IoError, sync::OnceLock};

use futures::{Sink, SinkExt, Stream, StreamExt};
use http::StatusCode;
use models::{
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
	AppResponse,
	prelude::*,
	utils::{False, Headers, WebSocketUpgrade},
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};
use tokio_tungstenite::tungstenite::{
	Error as TungsteniteError,
	Message,
	client::IntoClientRequest,
};

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
) -> Result<AppResponse<E>, ApiErrorResponse>
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
		})) => Ok(AppResponse {
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

/// Open a streaming (websocket) request to the API. Returns a duplex handle
/// that is both a [`Stream`] of server messages and a [`Sink`] for client
/// messages, JSON-encoding each frame. Ported from the runner's client so the
/// CLI can drive the interactive deployment shell.
pub async fn stream_request<E, ServerMsg, ClientMsg>(
	request: ApiRequest<E>,
) -> Result<
	impl Stream<Item = Result<ServerMsg, ErrorType>> + Sink<ClientMsg, Error: Debug>,
	ApiErrorResponse,
>
where
	E: ApiEndpoint<RequestBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	ServerMsg: DeserializeOwned,
	ClientMsg: Serialize,
{
	let mut client_request = http::Uri::builder()
		.scheme(
			if super::constants::API_BASE_URL.starts_with("https") {
				"wss"
			} else {
				"ws"
			},
		)
		.authority(
			super::constants::API_BASE_URL
				.trim_start_matches("https://")
				.trim_start_matches("http://"),
		)
		.path_and_query(format!(
			"{}?{}",
			request.path,
			serde_qs::to_string(&request.query).map_err(|err| ApiErrorResponse {
				status_code: StatusCode::INTERNAL_SERVER_ERROR,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(&err),
					message: err.to_string(),
				},
			})?
		))
		.build()
		.map_err(|err| ApiErrorResponse {
			status_code: StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error(&err),
				message: err.to_string(),
			},
		})?
		.into_client_request()
		.map_err(|err| ApiErrorResponse {
			status_code: StatusCode::INTERNAL_SERVER_ERROR,
			body: ApiErrorResponseBody {
				success: False,
				error: ErrorType::server_error(&err),
				message: err.to_string(),
			},
		})?;
	for (header, value) in &request.headers.to_header_map() {
		client_request
			.headers_mut()
			.insert(header.clone(), value.clone());
	}
	*client_request.method_mut() = E::METHOD;

	let stream = tokio_tungstenite::connect_async(client_request)
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
					error: ErrorType::server_error(err.to_string()),
					message: err.to_string(),
				},
			},
		})?
		.0
		.filter_map(async |msg| match msg {
			Ok(Message::Text(text)) => Some(
				serde_json::from_str(text.as_str())
					.inspect_err(|err| warn!("Error parsing text as JSON: {}", err))
					.map_err(ErrorType::server_error),
			),
			Ok(Message::Binary(bin)) => Some(
				serde_json::from_slice(bin.as_ref())
					.inspect_err(|err| warn!("Error parsing binary as JSON: {}", err))
					.map_err(ErrorType::server_error),
			),
			Ok(_) => None,
			Err(err) => {
				warn!("Error from websocket stream: {}", err);
				Some(Err(ErrorType::server_error(err)))
			}
		})
		.with(async |message| {
			Ok::<Message, TungsteniteError>(Message::Binary(
				serde_json::to_vec(&message)
					.inspect_err(|err| warn!("Error serializing message to JSON: {}", err))
					.map_err(|err| TungsteniteError::Io(IoError::other(err)))?
					.into(),
			))
		});

	Ok(stream)
}

/// Initialize a reqwest client that can be used across the application to make
/// requests
fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}
