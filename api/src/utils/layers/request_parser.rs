use std::{
	convert::Infallible,
	future::Future,
	marker::PhantomData,
	task::{Context, Poll},
};

use axum::{
	body::Body,
	extract::{Path, Query},
	http::Request,
	response::{IntoResponse, Response},
	RequestExt,
};
use models::{
	prelude::*,
	utils::{FromAxumRequest, Headers, IntoAxumResponse},
	ApiErrorResponse,
};
use tower::{Layer, Service};

use crate::{app::AppResponse, prelude::*};

/// A [`tower::Layer`] that can be used to parse the request and call the inner
/// service with the parsed request. Ideally, this will automatically be done by
/// [`RouterExt::mount_endpoint`], and you should not need to use this directly.
#[derive(Clone, Debug)]
pub struct RequestParserLayer<E>
where
	E: ApiEndpoint,
{
	phantom: PhantomData<E>,
	state: AppState,
}

impl<E> RequestParserLayer<E>
where
	E: ApiEndpoint,
{
	/// Create a new instance of the [`RequestParserLayer`] with the given
	/// state. This state will be used to parse the request, create a database
	/// transaction, and call the inner service. If the inner service fails, the
	/// database transaction will be automatically rolled back, otherwise it
	/// will be committed.
	pub fn with_state(state: AppState) -> Self {
		Self {
			phantom: PhantomData,
			state,
		}
	}
}

impl<S, E> Layer<S> for RequestParserLayer<E>
where
	for<'a> S: Service<AppRequest<'a, E>>,
	E: ApiEndpoint,
{
	type Service = RequestParser<S, E>;

	fn layer(&self, inner: S) -> Self::Service {
		RequestParser {
			inner,
			state: self.state.clone(),
			phantom: PhantomData,
		}
	}
}

/// A [`tower::Service`] that can be used to parse the request and call the
/// inner service with the parsed request. Ideally, this will automatically be
/// done by [`RouterExt::mount_endpoint`], and you should not need to use this
/// directly.
#[derive(Clone, Debug)]
pub struct RequestParser<S, E>
where
	for<'a> S: Service<AppRequest<'a, E>>,
	E: ApiEndpoint,
{
	inner: S,
	state: AppState,
	phantom: PhantomData<E>,
}

impl<S, E> Service<Request<Body>> for RequestParser<S, E>
where
	E: ApiEndpoint,
	for<'a> S: Service<AppRequest<'a, E>, Response = AppResponse<E>, Error = ErrorType> + Clone,
{
	type Response = Response;
	type Error = Infallible;
	type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner
			.poll_ready(cx)
			.map_err(|_| unreachable!("Layers must always be ready"))
	}

	#[instrument(skip(self, req))]
	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let mut state = self.state.clone();
		let mut inner = self.inner.clone();
		async {
			debug!("Parsing request for URL: {}", req.uri());

			let Ok(Path(path)) = req.extract_parts().await else {
				debug!("Failed to parse path: {}", req.uri().path());
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Request URL",
				)
				.into_response());
			};

			let Ok(Query(query)) = req.extract_parts().await else {
				debug!("Failed to parse query: {:?}", req.uri().query());
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Query Parameters",
				)
				.into_response());
			};

			let Some(headers) = <E::RequestHeaders as Headers>::from_header_map(req.headers())
			else {
				debug!("Failed to parse headers");
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid Headers",
				)
				.into_response());
			};

			let Ok(body) =
				<<E as ApiEndpoint>::RequestBody as FromAxumRequest>::from_axum_request(req).await
			else {
				debug!("Failed to parse body");
				return Ok(ApiErrorResponse::error_with_message(
					ErrorType::WrongParameters,
					"Invalid body",
				)
				.into_response());
			};

			debug!("Request parsed successfully");

			let mut redis = &mut state.redis;

			let Ok(mut database) = state.database.begin().await else {
				debug!("Failed to begin database transaction");
				return Ok(ApiErrorResponse::internal_error(
					"unable to begin database transaction",
				)
				.into_response());
			};

			let req = AppRequest {
				request: ApiRequest {
					path,
					query,
					headers,
					body,
				},
				database: &mut database,
				redis: &mut redis,
				config: state.config.clone(),
			};

			info!("Calling inner service");

			match inner.call(req).await {
				Ok(response) => {
					info!("Inner service called successfully");
					let Ok(()) = database.commit().await else {
						debug!("Failed to commit database transaction");
						return Ok(ApiErrorResponse::internal_error(
							"unable to commit database transaction",
						)
						.into_response());
					};
					Ok((
						response.status_code,
						response.headers.to_header_map(),
						response.body.into_axum_response(),
					)
						.into_response())
				}
				Err(error) => {
					warn!("Inner service failed: {:?}", error);
					let Ok(()) = database.rollback().await else {
						debug!("Failed to rollback database transaction");
						return Ok(ApiErrorResponse::internal_error(
							"unable to rollback database transaction",
						)
						.into_response());
					};

					Ok(ApiErrorResponse::error(error).into_response())
				}
			}
		}
	}
}
