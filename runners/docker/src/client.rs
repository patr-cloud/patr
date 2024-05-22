use futures::{Stream, StreamExt};
use http::StatusCode;
use httparse::{Header, Request};
use models::{
	prelude::*,
	utils::{False, Headers, WebSocketUpgrade},
	ApiErrorResponse,
	ApiErrorResponseBody,
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};

pub async fn stream_request<E, ServerMsg, ClientMsg>(
	request: ApiRequest<E>,
) -> Result<impl Stream<Item = ServerMsg>, ApiErrorResponse>
where
	E: ApiEndpoint<RequestBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
	<E::RequestBody as Preprocessable>::Processed: Send,
	ServerMsg: DeserializeOwned,
	ClientMsg: Serialize,
{
	Ok(tokio_tungstenite::connect_async(Request {
		method: Some(&E::METHOD.to_string()),
		path: Some(&request.path.to_string()),
		version: Some(1),
		headers: &mut request
			.headers
			.to_header_map()
			.iter()
			.map(|(k, v)| Header {
				name: k.as_str(),
				value: v.as_bytes(),
			})
			.collect::<Vec<_>>(),
	})
	.await
	.map_err(|err| match err {
		TungsteniteError::Http(err) => {
			let (parts, body) = err.into_parts();
			ApiErrorResponse {
				status_code: parts.status,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::server_error(String::from_utf8_lossy(
						body.as_deref().unwrap_or_default(),
					)),
					message: String::from_utf8_lossy(body.as_deref().unwrap_or_default())
						.to_string(),
				},
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
		let msg = msg.ok()?;
		let msg = match msg {
			Message::Text(text) => text,
			_ => return None,
		};
		let msg = serde_json::from_str(&msg).ok()?;
		Some(msg)
	}))
}

fn initialize_client() -> Client {
	Client::builder()
		.build()
		.expect("failed to initialize client")
}
