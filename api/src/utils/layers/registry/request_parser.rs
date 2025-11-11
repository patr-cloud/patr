//! Registry request parser layer.
//!
//! This layer parses incoming HTTP requests into structured components for
//! registry endpoints. It extracts path parameters, query parameters, headers,
//! and preserves the streaming body without buffering it in memory.
//!
//! The layer is generic over the endpoint type [`E`] which implements
//! [`RegistryEndpoint`], allowing type-safe parsing of request components
//! specific to each endpoint.

use std::{
	convert::Infallible,
	future::Future,
	marker::PhantomData,
	net::IpAddr,
	task::{Context, Poll},
};

use axum::{
	RequestExt,
	body::Body,
	extract::Path,
	http::Request,
	response::{IntoResponse, Response},
};
use models::utils::Headers;
use oci_spec::distribution::ErrorCode;
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::{routes::registry_patr_cloud::prelude::*, utils::extractors::ClientIP};

/// Tower layer that parses HTTP requests into registry request components.
///
/// This layer extracts:
/// - Path parameters using `TypedPath`
/// - Query parameters from the URL query string
/// - Request headers
/// - Streaming body (without buffering)
///
/// If parsing fails, it returns an OCI-compliant error response.
#[derive(Clone)]
pub struct RegistryRequestParserLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	phantom: PhantomData<E>,
}

impl<E> RegistryRequestParserLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	/// Create a new registry request parser layer.
	pub const fn new() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RegistryRequestParserLayer<E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Service = RegistryRequestParserService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RegistryRequestParserService {
			inner,
			phantom: PhantomData,
		}
	}
}

/// Tower service that parses HTTP requests into registry request components.
///
/// This service is created by `RegistryRequestParserLayer` and handles the
/// actual parsing logic. It converts raw HTTP requests into structured
/// `ParsedRegistryRequest` objects that can be consumed by downstream layers.
#[derive(Clone)]
pub struct RegistryRequestParserService<S, E>
where
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	inner: S,
	phantom: PhantomData<E>,
}

impl<S, E> Service<Request<Body>> for RegistryRequestParserService<S, E>
where
	S: Service<
			(RegistryUnprocessedApiRequest<E>, IpAddr),
			Response = RegistryResponse<E>,
			Error = RegistryError,
		> + Clone,
	E: RegistryEndpoint,
	<E::RequestPath as Preprocessable>::Processed: Send,
	<E::RequestQuery as Preprocessable>::Processed: Send,
{
	type Error = Infallible;
	type Response = Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "RegistryRequestParserService", fields(method = %req.method(), uri = %req.uri()))]
	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let mut inner = self.inner.clone();
		async move {
			debug!("Parsing registry request for URL: {}", req.uri());

			// Extract path parameters
			// We need to manually extract the path because TypedPath requires state
			// The path is already matched by the router, so we just need to deserialize it
			let Ok(Path(path)) = req.extract_parts().await.inspect_err(|err| {
				debug!("Failed to parse path `{}`: {}", req.uri().path(), err);
			}) else {
				return Ok(RegistryError::name_invalid(format!(
					"Invalid request path: {}",
					req.uri().path()
				))
				.into_response());
			};

			// Parse query parameters from URL query string
			let Ok(query) = serde_urlencoded::from_str(req.uri().query().unwrap_or_default())
				.inspect_err(|err| {
					debug!("Failed to parse query `{:?}`: {}", req.uri().query(), err);
				})
			else {
				return Ok(RegistryError::new(
					ErrorCode::Unsupported,
					format!("Invalid Query Parameters"),
				)
				.into_response());
			};

			// Parse headers from request header map
			let Ok(headers) = <E::RequestHeaders as Headers>::from_header_map(req.headers())
				.inspect_err(|err| {
					debug!("Failed to parse headers: {err}");
				})
			else {
				return Ok(
					RegistryError::new(ErrorCode::Unsupported, format!("Invalid Headers"))
						.into_response(),
				);
			};

			let Ok(ClientIP(client_ip)) = req.extract_parts().await;

			// Extract the body without buffering it
			// The body is a streaming type that will be consumed by handlers
			let body = req.into_body();

			debug!("Request parsed successfully");

			// Create the parsed request object
			let request = RegistryUnprocessedApiRequest {
				path,
				query,
				headers,
				body,
			};

			info!("Calling inner service");

			let response = inner
				.call((request, client_ip))
				.await
				.inspect(|_| info!("Inner service called successfully"))
				.map(|response| {
					(
						response.status_code,
						response.headers.to_header_map(),
						response.body,
					)
						.into_response()
				})
				.unwrap_or_else(|error| error.into_response());

			Ok(response)
		}
	}
}
