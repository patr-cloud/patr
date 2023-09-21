use std::{
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Sink, Stream};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{
	tungstenite::Message,
	MaybeTlsStream,
	WebSocketStream as TokioWebSocketStream,
};

use crate::prelude::*;

pub struct WebSocketStream<ServerMsg, ClientMsg> {
	stream: TokioWebSocketStream<MaybeTlsStream<TcpStream>>,
	marker: std::marker::PhantomData<(ServerMsg, ClientMsg)>,
}

impl<ServerMsg, ClientMsg> WebSocketStream<ServerMsg, ClientMsg> {
	pub fn new(stream: TokioWebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
		Self {
			stream,
			marker: std::marker::PhantomData,
		}
	}
}

impl<ServerMsg, ClientMsg> Stream for WebSocketStream<ServerMsg, ClientMsg>
where
	ServerMsg: DeserializeOwned,
	ClientMsg: Serialize,
	Self: Unpin,
{
	type Item = Result<ServerMsg, ErrorType>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		match Pin::new(&mut self.stream)
			.poll_next(cx)
			.map_err(|err| ErrorType::InternalServerError(err.into()))
			.map_ok(|msg| {
				msg.into_text()
					.map_err(|err| ErrorType::InternalServerError(err.into()))
					.and_then(|text| {
						serde_json::from_str(&text)
							.map_err(|err| ErrorType::InternalServerError(err.into()))
					})
			}) {
			Poll::Ready(Some(Ok(msg))) => Poll::Ready(Some(msg)),
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl<ServerMsg, ClientMsg> Sink<ClientMsg> for WebSocketStream<ServerMsg, ClientMsg>
where
	ServerMsg: DeserializeOwned,
	ClientMsg: Serialize,
	Self: Unpin,
{
	type Error = ErrorType;

	fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.stream)
			.poll_ready(cx)
			.map_err(|err| ErrorType::InternalServerError(err.into()))
	}

	fn start_send(mut self: Pin<&mut Self>, item: ClientMsg) -> Result<(), Self::Error> {
		Pin::new(&mut self.stream)
			.start_send(Message::Text(
				serde_json::to_string(&item).expect("unable to serialize item"),
			))
			.map_err(|err| ErrorType::InternalServerError(err.into()))
	}

	fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.stream)
			.poll_flush(cx)
			.map_err(|err| ErrorType::InternalServerError(err.into()))
	}

	fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Pin::new(&mut self.stream)
			.poll_flush(cx)
			.map_err(|err| ErrorType::InternalServerError(err.into()))
	}
}
