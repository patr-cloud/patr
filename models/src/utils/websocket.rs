use std::future::Future;

use axum::{body::Body, http::Request, RequestExt};
use axum_typed_websockets::WebSocketUpgrade as TypedWebSocketUpgrade;

use super::FromAxumRequest;
use crate::ErrorType;

/// A websocket upgrade request. This can be used as a body type for websocket
/// endpoints.
pub struct WebSocketUpgrade<ServerMsg, ClientMsg>(pub TypedWebSocketUpgrade<ServerMsg, ClientMsg>);

impl<ServerMsg, ClientMsg> FromAxumRequest for WebSocketUpgrade<ServerMsg, ClientMsg>
where
	ClientMsg: 'static,
	ServerMsg: 'static,
{
	#[tracing::instrument(skip(request))]
	fn from_axum_request(request: Request<Body>) -> impl Future<Output = Result<Self, ErrorType>> {
		async move {
			request
				.extract()
				.await
				.map(WebSocketUpgrade)
				.map_err(|err| ErrorType::server_error(err.to_string()))
		}
	}
}
