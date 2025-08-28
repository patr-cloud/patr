use std::{
	net::IpAddr,
	task::{Context, Poll},
};

use axum::{body::Body, http::Request};
use tower::{Layer, Service};

use crate::prelude::*;

pub struct RegistryRequest<'a> {
	pub request: Request<Body>,
	pub database: &'a mut DatabaseTransaction,
	pub client_ip: IpAddr,
}

/// A [`tower::Layer`] used to convert types
#[derive(Clone)]
pub struct RegistryRequestParserLayer {}

impl RegistryRequestParserLayer {
	pub fn new() -> Self {
		Self {}
	}
}

impl<S> Layer<S> for RegistryRequestParserLayer {
	type Service = RegistryRequestParserService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryRequestParserService { inner }
	}
}

/// A [`tower::Service`] to convert types
#[derive(Clone)]
pub struct RegistryRequestParserService<S> {
	inner: S,
}

impl<S> Service<Request<Body>> for RegistryRequestParserService<S>
where
	S: Service<Request<Body>> + Clone,
{
	type Error = S::Error;
	type Response = S::Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let mut inner = self.inner.clone();

		async move {
			debug!("Parsing request for URL: {}", req.uri());
			let response = inner
				.call(req)
				.await
				.inspect(|_| info!("Inner Service Called Successfully"));

			response
		}
	}
}
