use futures::{Stream, StreamExt};
use http::StatusCode;
use models::{
	prelude::*,
	utils::{constants, False, Headers, WebSocketUpgrade},
	ApiErrorResponse,
	ApiErrorResponseBody,
};
use preprocess::Preprocessable;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::{
	client::IntoClientRequest,
	Error as TungsteniteError,
	Message,
};

use crate::prelude::*;

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
		request.path.to_string()
	)
	.into_client_request()
	.map_err(|err| ApiErrorResponse {
		status_code: StatusCode::INTERNAL_SERVER_ERROR,
		body: ApiErrorResponseBody {
			success: False,
			error: ErrorType::server_error(err.to_string()),
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
					error: ErrorType::server_error(err.to_string()),
					message: err.to_string(),
				},
			},
		})?
		.0
		.filter_map(|msg| async move {
			let msg = msg
				.inspect_err(|err| warn!("Error from websocket stream: {}", err))
				.ok()?;
			let msg = match msg {
				Message::Text(text) => text,
				_ => return None,
			};
			let msg = serde_json::from_str(&msg)
				.inspect_err(|err| warn!("Error parsing text as JSON: {}", err))
				.ok()?;
			Some(msg)
		}))
}
