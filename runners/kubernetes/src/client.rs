use std::{str::FromStr, sync::OnceLock};

use futures::{Stream, StreamExt};
use http::{HeaderName, HeaderValue, StatusCode};
use httparse::{Header, Request};
use models::{
	prelude::*,
	utils::{False, Headers, WebSocketUpgrade},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiResponseBody,
	ApiSuccessResponseBody,
};
use preprocess::Preprocessable;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
use url::Url;

use crate::prelude::*;

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
			reqwest::Method::from_str(&E::METHOD.to_string()).unwrap(),
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
	E: ApiEndpoint<ResponseBody = WebSocketUpgrade<ServerMsg, ClientMsg>>,
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
