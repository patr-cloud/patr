use std::sync::OnceLock;

use futures::{Stream, StreamExt};
use httparse::{Header, Request};
use models::{
	utils::{Headers, WebSocketUpgrade},
	ApiEndpoint,
	ApiRequest,
	ApiResponseBody,
	AppResponse,
	ErrorType,
};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::Message;

static CLIENT: OnceLock<Client> = OnceLock::new();

pub async fn make_request<E>(request: ApiRequest<E>) -> Result<AppResponse<E>, ErrorType>
where
	E: ApiEndpoint,
	E::ResponseBody: DeserializeOwned,
	E::RequestBody: Serialize,
{
	let body = serde_json::to_value(request.body)
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;
	let mut reqwest = CLIENT
		.get_or_init(|| Client::new())
		.request(E::METHOD, request.path.to_string())
		.headers(request.headers.to_header_map())
		.query(&request.query);

	if !body.is_null() {
		reqwest = reqwest.json(&body);
	}

	let response = CLIENT
		.get_or_init(|| Client::new())
		.execute(
			reqwest
				.build()
				.map_err(|err| err.to_string())
				.map_err(ErrorType::server_error)?,
		)
		.await
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;

	let status_code = response.status();
	let headers = <E::ResponseHeaders as Headers>::from_header_map(&response.headers())
		.ok_or_else(|| ErrorType::server_error("Failed to parse response headers"))?;

	let body = response
		.json::<ApiResponseBody<E::ResponseBody>>()
		.await
		.map_err(|err| err.to_string())
		.map_err(ErrorType::server_error)?;

	match body {
		ApiResponseBody::Success(data) => Ok(AppResponse::builder()
			.status_code(status_code)
			.headers(headers)
			.body(data.response)
			.build()),
		ApiResponseBody::Error(body) => Err(body.error),
	}
}

pub async fn stream_request<E, ServerMsg, ClientMsg>(
	request: ApiRequest<E>,
) -> Result<impl Stream<Item = ServerMsg>, ErrorType>
where
	E: ApiEndpoint<ResponseBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
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
	.map_err(|err| err.to_string())
	.map_err(ErrorType::server_error)?
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
