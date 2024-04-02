use axum::{body::Body, http::Request};
#[cfg(not(target_arch = "wasm32"))]
use axum_typed_websockets::WebSocketUpgrade as TypedWebSocketUpgrade;
use preprocess::Preprocessable;

use super::{FromAxumRequest, RequiresRequestHeaders, RequiresResponseHeaders};
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
		use axum::RequestExt;

		request
			.extract()
			.await
			.map(WebSocketUpgrade)
			.map_err(ErrorType::server_error)
	}
}

#[cfg(target_arch = "wasm32")]
impl<ServerMsg, ClientMsg> FromAxumRequest for WebSocketUpgrade<ServerMsg, ClientMsg>
where
	ClientMsg: 'static,
	ServerMsg: 'static,
{
	#[tracing::instrument(skip(_request))]
	async fn from_axum_request(_request: Request<Body>) -> Result<Self, ErrorType> {
		Ok(Self(std::marker::PhantomData))
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
