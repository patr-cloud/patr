use std::{
	convert::Infallible,
	future::Future,
	marker::PhantomData,
	net::IpAddr,
	task::{Context, Poll},
};

use axum::{
	body::Body,
	extract::Path,
	http::Request,
	response::{IntoResponse, Response},
	RequestExt,
};
use models::{
	prelude::*,
	utils::{FromAxumRequest, GenericResponse, Headers, IntoAxumResponse},
	ApiErrorResponse,
};
use preprocess::Preprocessable;
use tower::{Layer, Service};

use crate::utils::extractors::ClientIP;

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_endpoint`], and you should not need to use this directly.
#[derive(Clone, Debug, Default)]
pub struct RequestParserLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The endpoint type that this layer will handle.
	phantom: PhantomData<E>,
}

impl<E> RequestParserLayer<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// Create a new instance of the [`RequestParserLayer`]
	pub const fn new() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl<S, E> Layer<S> for RequestParserLayer<E>
where
	for<'a> S: Service<(ApiRequest<E>, IpAddr)>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Service = RequestParserService<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RequestParserService {
			inner,
			phantom: PhantomData,
		}
	}
}

/// A [`tower::Service`] that can be used to parse the request and call the
/// inner service with the parsed request. Ideally, this will automatically be
/// done by [`RouterExt::mount_endpoint`], and you should not need to use this
/// directly.
#[derive(Clone, Debug)]
pub struct RequestParserService<S, E>
where
	for<'a> S: Service<(ApiRequest<E>, IpAddr)>,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The inner service that will be called with the parsed request.
	inner: S,
	/// The endpoint type that this service will handle.
	phantom: PhantomData<E>,
}

impl<S, E> Service<Request<Body>> for RequestParserService<S, E>
where
	for<'a> S:
		Service<(ApiRequest<E>, IpAddr), Response = AppResponse<E>, Error = ErrorType> + Clone,
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = Infallible;
	type Response = Response;

	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req), name = "RequestParserService")]
	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let mut inner = self.inner.clone();
		async {
			debug!("Parsing request for URL: {}", req.uri());

			let Ok(Path(path)) = req.extract_parts().await.inspect_err(|err| {
				debug!("Failed to parse path `{}`: {}", req.uri().path(), err);
			}) else {
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Request URL",
				)
				.into_response());
			};

			let Ok(query) = serde_urlencoded::from_str(req.uri().query().unwrap_or_default())
				.inspect_err(|err| {
					debug!("Failed to parse query `{:?}`: {}", req.uri().query(), err);
				})
			else {
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Query Parameters",
				)
				.into_response());
			};

			let Ok(headers) = <E::RequestHeaders as Headers>::from_header_map(req.headers())
				.inspect_err(|err| {
					debug!("Failed to parse headers: {err}");
				})
			else {
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Headers",
				)
				.into_response());
			};

			let Ok(ClientIP(client_ip)) = req.extract_parts().await;

			let Ok(body) =
				<<E as ApiEndpoint>::RequestBody as FromAxumRequest>::from_axum_request(req)
					.await
					.inspect_err(|err| debug!("Error parsing body: {}", err.to_string()))
			else {
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid body",
				)
				.into_response());
			};

			debug!("Request parsed successfully");

			let request = ApiRequest {
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
					if response.body.is::<GenericResponse>() {
						response.body.into_axum_response()
					} else {
						(
							response.status_code,
							response.headers.to_header_map(),
							response.body.into_axum_response(),
						)
							.into_response()
					}
				})
				.unwrap_or_else(|error| {
					if let ErrorType::InternalServerError = &error {
						error!("Internal server error: {}", error);
					} else {
						warn!("Inner service failed: {:?}", error);
					}
					ApiErrorResponse::error(error).into_response()
				});

			Ok(response)
		}
	}
}
