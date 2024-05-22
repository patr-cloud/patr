use std::str::FromStr;

use futures::StreamExt;
use http::StatusCode;
use httparse::{Header, Request};
use models::{
	api::workspace::runner::*,
	prelude::*,
	utils::{False, Headers},
	ApiErrorResponse,
	ApiErrorResponseBody,
};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};

mod client;

#[tokio::main]
async fn main() {
	tokio_tungstenite::connect_async(Request {
		method: Some(&StreamRunnerDataForWorkspaceRequest::METHOD.to_string()),
		path: Some(&format!(
			"ws://localhost:3000/{}",
			StreamRunnerDataForWorkspacePath {
				workspace_id: Uuid::parse_str("00000000000000000000000000000000").unwrap(),
				runner_id: Uuid::parse_str("00000000000000000000000000000000").unwrap(),
			}
			.to_string()
		)),
		version: Some(1),
		headers: &mut StreamRunnerDataForWorkspaceRequestHeaders {
			authorization: BearerToken::from_str("Bearer fpKL54jvWmEGVoRdCNjG").unwrap(),
			user_agent: UserAgent::from_str("runner/docker").unwrap(),
		}
		.to_header_map()
		.iter()
		.map(|(k, v)| Header {
			name: k.as_str(),
			value: v.as_bytes(),
		})
		.chain([
			Header {
				name: "sec-websocket-key",
				value: tokio_tungstenite::tungstenite::handshake::client::generate_key().as_bytes(),
			},
			Header {
				name: "sec-websocket-version",
				value: b"13",
			},
			Header {
				name: "host",
				value: b"localhost:3000",
			},
			Header {
				name: "upgrade",
				value: b"websocket",
			},
			Header {
				name: "connection",
				value: b"Upgrade",
			},
		])
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
	})
	.unwrap()
	.0
	.filter_map(|msg| async move {
		let msg = msg.ok()?;
		let msg = match msg {
			Message::Text(text) => text,
			_ => return None,
		};
		let msg = serde_json::from_str(&msg).ok()?;
		Some(msg)
	})
	.for_each_concurrent(4, |response| async move {
		println!("{:#?}", response);
	})
	.await;
}
