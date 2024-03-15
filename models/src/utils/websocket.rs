#[cfg(not(target_arch = "wasm32"))]
use axum::{body::Body, http::Request, RequestExt};
#[cfg(not(target_arch = "wasm32"))]
use axum_typed_websockets::WebSocketUpgrade as TypedWebSocketUpgrade;
use preprocess::Preprocessable;

#[cfg(not(target_arch = "wasm32"))]
use super::FromAxumRequest;
use super::{RequiresRequestHeaders, RequiresResponseHeaders};
#[cfg(not(target_arch = "wasm32"))]
use crate::ErrorType;

/// A websocket upgrade request. This can be used as a body type for websocket
/// endpoints.
pub struct WebSocketUpgrade<ServerMsg, ClientMsg>(
	#[cfg(not(target_arch = "wasm32"))] pub TypedWebSocketUpgrade<ServerMsg, ClientMsg>,
	#[cfg(target_arch = "wasm32")] pub std::marker::PhantomData<(ServerMsg, ClientMsg)>,
);

#[cfg(not(target_arch = "wasm32"))]
impl<ServerMsg, ClientMsg> FromAxumRequest for WebSocketUpgrade<ServerMsg, ClientMsg>
where
	ClientMsg: 'static,
	ServerMsg: 'static,
{
	#[tracing::instrument(skip(request))]
	async fn from_axum_request(request: Request<Body>) -> Result<Self, ErrorType> {
		request
			.extract()
			.await
			.map(WebSocketUpgrade)
			.map_err(ErrorType::server_error)
	}
}

impl<ServerMsg, ClientMsg> Preprocessable for WebSocketUpgrade<ServerMsg, ClientMsg> {
	type Processed = Self;

	fn preprocess(self) -> Result<Self, preprocess::Error> {
		Ok(self)
	}
}

impl<ServerMsg, ClientMsg> RequiresRequestHeaders for WebSocketUpgrade<ServerMsg, ClientMsg> {
	type RequiredRequestHeaders = ();
}

impl<ServerMsg, ClientMsg> RequiresResponseHeaders for WebSocketUpgrade<ServerMsg, ClientMsg> {
	type RequiredResponseHeaders = ();
}
